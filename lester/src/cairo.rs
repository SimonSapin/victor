use crate::cairo_ffi::*;
use crate::convert::TryInto;
use crate::errors::{CairoError, LesterError};
use std::any::Any;
use std::fs;
use std::io::{self, Read, Write};
use std::mem;
use std::os::raw::*;
use std::panic;
use std::path;
use std::slice;

/// The pixels from an `ImageSurface`
#[derive(PartialEq, Eq)]
pub struct Argb32Pixels<'data> {
    pub width: usize,
    pub height: usize,

    /// A slice of length `width * height` containing the image’s pixels.
    /// The pixel at position `(x, y)` is at index `x + width * y`.
    ///
    /// A pixel’s upper 8 bits is the alpha channel if the image is in ARGB32 format,
    /// or undefined for the RGB24 format.
    /// The next three groups of 8 bits (upper to lower) are the red, green, and blue channels.
    pub buffer: &'data mut [u32],
}

/// A cairo “image surface”: an in-memory pixel buffer.
///
/// Only the RGB24 and ARGB32 pixel formats (which have compatible memory representation)
/// are supported.
pub struct ImageSurface {
    ptr: *mut cairo_surface_t,
}

impl Drop for ImageSurface {
    fn drop(&mut self) {
        unsafe {
            cairo_surface_destroy(self.ptr);
        }
    }
}

impl ImageSurface {
    /// Create a new RGB24 image surface of the given size, in pixels
    pub fn new_rgb24(width: usize, height: usize) -> Result<Self, CairoError> {
        Self::new(CAIRO_FORMAT_RGB24, width, height)
    }

    /// Create a new ARGB32 image surface of the given size, in pixels
    pub fn new_argb32(width: usize, height: usize) -> Result<Self, CairoError> {
        Self::new(CAIRO_FORMAT_ARGB32, width, height)
    }

    fn new(format: cairo_format_t, width: usize, height: usize) -> Result<Self, CairoError> {
        Self::new_c_int(
            format,
            width.try_into().unwrap(),
            height.try_into().unwrap(),
        )
    }

    pub(crate) fn new_c_int(
        format: cairo_format_t,
        width: c_int,
        height: c_int,
    ) -> Result<Self, CairoError> {
        unsafe {
            let ptr = cairo_image_surface_create(format, width, height);
            let mut surface = ImageSurface { ptr };
            surface.check_status()?;
            Ok(surface)
        }
    }

    fn check_status(&mut self) -> Result<(), CairoError> {
        CairoError::check(unsafe { cairo_surface_status(self.ptr) })
    }

    pub(crate) fn context(&mut self) -> Result<CairoContext, CairoError> {
        unsafe {
            let mut context = CairoContext {
                ptr: cairo_create(self.ptr),
            };
            context.check_status()?;
            Ok(context)
        }
    }

    /// Access the pixels of this image surface
    pub fn pixels<'data>(&'data mut self) -> Argb32Pixels<'data> {
        unsafe {
            cairo_surface_flush(self.ptr);
            let data = cairo_image_surface_get_data(self.ptr);
            let width = cairo_image_surface_get_width(self.ptr);
            let height = cairo_image_surface_get_height(self.ptr);
            let stride = cairo_image_surface_get_stride(self.ptr);
            let format = cairo_image_surface_get_format(self.ptr);
            assert!(
                format == CAIRO_FORMAT_RGB24 || format == CAIRO_FORMAT_ARGB32,
                "Unsupported pixel format"
            );

            // In theory we shouldn’t rely on this.
            // In practice cairo picks a stride that is `width * size_of_pixel`
            // rounded up to 32 bits.
            // ARGB32 and RGB24 both use 32 bit per pixel, so rounding is a no-op.
            assert!(
                stride == width * (mem::size_of::<u32>() as i32),
                "Expected 32bit pixel to make width satisfy stride requirements"
            );

            assert!(
                (data as usize) % mem::align_of::<u32>() == 0,
                "Expected cairo to allocated data aligned to 32 bits"
            );

            Argb32Pixels {
                width: width.try_into().unwrap(),
                height: height.try_into().unwrap(),
                buffer: slice::from_raw_parts_mut(
                    data as *mut u32,
                    (width * height).try_into().unwrap(),
                ),
            }
        }
    }

    /// Read and decode a PNG image from the given file name and create an image surface for it.
    pub fn read_from_png_file<P: AsRef<path::Path>>(filename: P) -> Result<Self, LesterError> {
        Self::read_from_png(io::BufReader::new(fs::File::open(filename)?))
    }

    /// Encode this image to PNG and write it into the file with the given name.
    pub fn write_to_png_file<P: AsRef<path::Path>>(&self, filename: P) -> Result<(), LesterError> {
        self.write_to_png(io::BufWriter::new(fs::File::create(filename)?))
    }
}

// Private
pub(crate) struct CairoContext {
    pub(crate) ptr: *mut cairo_t,
}

impl CairoContext {
    pub(crate) fn check_status(&mut self) -> Result<(), CairoError> {
        CairoError::check(unsafe { cairo_status(self.ptr) })
    }

    pub(crate) fn set_source_rgb(&mut self, r: f64, g: f64, b: f64) {
        unsafe { cairo_set_source_rgb(self.ptr, r, g, b) }
    }

    pub(crate) fn paint(&mut self) {
        unsafe { cairo_paint(self.ptr) }
    }

    pub(crate) fn scale(&mut self, x: f64, y: f64) {
        unsafe {
            cairo_scale(self.ptr, x, y);
        }
    }
}

impl Drop for CairoContext {
    fn drop(&mut self) {
        unsafe {
            cairo_destroy(self.ptr);
        }
    }
}

macro_rules! with_c_callback {
    (
        $stream: ident : $StreamType: ty : $StreamTrait: ident;
        fn callback($($closure_args: tt)*) -> $ErrorConst: ident $body: block
        ($wrap: expr)($function: ident($($function_args: tt)*))
    ) => {{
        struct ClosureData<Stream> {
            stream: Stream,
            stream_result: Result<(), io::Error>,
            panic_payload: Option<Box<Any + Send + 'static>>
        };
        let mut closure_data = ClosureData {
            stream: $stream,
            stream_result: Ok(()),
            panic_payload: None,
        };
        let closure_data_ptr: *mut ClosureData<$StreamType> = &mut closure_data;

        unsafe extern "C" fn callback<Stream: $StreamTrait>(
            closure_data_ptr: *mut c_void, $($closure_args)*
        ) -> cairo_status_t {
            let panic_result = panic::catch_unwind(|| {
                let closure_data = &mut *(closure_data_ptr as *mut ClosureData<Stream>);
                if closure_data.stream_result.is_err() {
                    return $ErrorConst
                }

                let $stream = &mut closure_data.stream;
                match $body {
                    Ok(()) => {
                        CAIRO_STATUS_SUCCESS
                    }
                    Err(error) => {
                        closure_data.stream_result = Err(error);
                        $ErrorConst
                    }
                }
            });
            match panic_result {
                Ok(value) => value,
                Err(panic_payload) => {
                    let closure_data = &mut *(closure_data_ptr as *mut ClosureData<Stream>);
                    closure_data.panic_payload = Some(panic_payload);
                    $ErrorConst
                }
            }
        }

        let result = unsafe {
            $wrap($function(
                $($function_args)*
                callback::<$StreamType>,
                closure_data_ptr as *mut c_void
            ))
        };
        if let Some(panic_payload) = closure_data.panic_payload {
            panic::resume_unwind(panic_payload)
        }
        closure_data.stream_result?;
        result
    }}
}

impl ImageSurface {
    /// Read and decode a PNG image from the given stream and create an image surface for it.
    ///
    /// Note: this may do many read calls.
    /// If a stream is backed by costly system calls (such as `File` or `TcpStream`),
    /// this constructor will likely perform better with that stream wrapped in `BufReader`.
    ///
    /// See also the `read_from_png_file` method.
    pub fn read_from_png<R: Read>(stream: R) -> Result<Self, LesterError> {
        let mut surface = with_c_callback! {
            stream: R: Read;
            fn callback(buffer: *mut c_uchar, length: c_uint) -> CAIRO_STATUS_READ_ERROR {
                let slice = slice::from_raw_parts_mut(buffer, length.try_into().unwrap());
                stream.read_exact(slice)
            }
            (|ptr| ImageSurface { ptr })(cairo_image_surface_create_from_png_stream())
        };

        surface.check_status()?;
        Ok(surface)
    }

    /// Encode this image to PNG and write it to the given stream.
    ///
    /// Note: this may do many write calls.
    /// If a stream is backed by costly system calls (such as `File` or `TcpStream`),
    /// this method will likely perform better with that stream wrapped in `BufWriter`.
    ///
    /// See also the `write_to_png_file` method.
    pub fn write_to_png<W: Write>(&self, stream: W) -> Result<(), LesterError> {
        let status = with_c_callback! {
            stream: W: Write;
            fn callback(buffer: *const c_uchar, length: c_uint) -> CAIRO_STATUS_WRITE_ERROR {
                let slice = slice::from_raw_parts(buffer, length.try_into().unwrap());
                stream.write_all(slice)
            }
            (|s| s)(cairo_surface_write_to_png_stream(self.ptr,))
        };

        CairoError::check(status)?;
        Ok(())
    }
}
