use cairo_ffi::*;
use std::any::Any;
use std::error::Error as StdError;
use std::ffi::CStr;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::mem;
use std::os::raw::*;
use std::panic;
use std::path;
use std::slice;
use poppler_ffi::*;

mod cairo_ffi;  // Not public or re-exported
mod poppler_ffi;  // Not public or re-exported

pub struct PdfDocument<'data> {
    ptr: *mut PopplerDocument,
    phantom: PhantomData<&'data [u8]>,
}

impl<'data> PdfDocument<'data> {
    pub fn from_bytes(bytes: &'data [u8]) -> Result<Self, GlibError> {
        let mut error = 0 as *mut GError;
        let ptr = unsafe {
            poppler_document_new_from_data(
                // Although this function takes *mut c_char rather than *const c_char,
                // that pointer is only passed to Poppler’s `MemStream` abstraction
                // which appears to only provide read access.
                bytes.as_ptr() as *const c_char as *mut c_char,
                bytes.len() as c_int,
                0 as *const c_char,
                &mut error
            )
        };
        if ptr.is_null() {
            Err(GlibError { ptr: error })
        } else {
            Ok(PdfDocument { ptr, phantom: PhantomData })
        }
    }

    pub fn page_count(&self) -> usize {
        unsafe {
            poppler_document_get_n_pages(self.ptr) as usize
        }
    }

    pub fn get_page<'doc>(&'doc self, index: usize) -> Option<PdfPage<'doc>> {
        let index = index as c_int;
        let ptr = unsafe {
            if poppler_document_get_n_pages(self.ptr) <= index {
                return None
            }
            poppler_document_get_page(self.ptr, index)
        };
        if ptr.is_null() {
            None
        } else {
            Some(PdfPage { ptr, phantom: PhantomData })
        }
    }
}

impl<'data> Drop for PdfDocument<'data> {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.ptr as *mut c_void)
        }
    }
}

pub struct PdfPage<'doc> {
    ptr: *mut PopplerPage,
    phantom: PhantomData<&'doc ()>,
}

impl<'doc> PdfPage<'doc> {
    pub fn size(&self) -> (f64, f64) {
        let mut width = 0.;
        let mut height = 0.;
        unsafe {
            poppler_page_get_size(self.ptr, &mut width, &mut height)
        }
        (width, height)
    }

    pub fn render_96dpi(&self, surface: &mut ImageSurface) -> Result<(), CairoError> {
        self.render(surface, 96., 96.)
    }

    pub fn render(&self, surface: &mut ImageSurface, dpi_x: f64, dpi_y: f64) -> Result<(), CairoError> {
        // PDF’s default unit is the PostScript point, wich is 1/72 inches.
        let scale_x = dpi_x / 72.;
        let scale_y = dpi_y / 72.;
        let context = surface.context()?;
        unsafe {
            cairo_scale(context.ptr, scale_x, scale_y);
            cairo_set_antialias(context.ptr, CAIRO_ANTIALIAS_NONE);
            poppler_page_render(self.ptr, context.ptr);
            cairo_surface_flush(surface.ptr);
        }
        context.check_status()?;
        Ok(())
    }
}

impl<'doc> Drop for PdfPage<'doc> {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.ptr as *mut c_void)
        }
    }
}

pub struct Argb32Image<'data> {
    pub width: usize,
    pub height: usize,
    pub pixels: &'data mut [u32],
}

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
    pub fn new_rgb24(width: usize, height: usize) -> Result<Self, CairoError> {
        Self::new(CAIRO_FORMAT_RGB24, width, height)
    }

    pub fn new_argb32(width: usize, height: usize) -> Result<Self, CairoError> {
        Self::new(CAIRO_FORMAT_ARGB32, width, height)
    }

    fn new(format: cairo_format_t, width: usize, height: usize) -> Result<Self, CairoError> {
        unsafe {
            let ptr = cairo_image_surface_create(format, width as _, height as _);
            let surface = ImageSurface { ptr };
            surface.check_status()?;
            Ok(surface)
        }
    }

    fn check_status(&self) -> Result<(), CairoError> {
        CairoError::check(unsafe { cairo_surface_status(self.ptr) })
    }

    fn context(&self) -> Result<CairoContext, CairoError> {
        unsafe {
            let context = CairoContext { ptr: cairo_create(self.ptr) };
            context.check_status()?;
            Ok(context)
        }
    }

    pub fn as_image<'data>(&'data mut self) -> Argb32Image<'data> {
        unsafe {
            let data = cairo_image_surface_get_data(self.ptr);
            let width = cairo_image_surface_get_width(self.ptr);
            let height = cairo_image_surface_get_height(self.ptr);
            let stride = cairo_image_surface_get_stride(self.ptr);
            let format = cairo_image_surface_get_format(self.ptr);
            assert!(format == CAIRO_FORMAT_RGB24 ||
                    format == CAIRO_FORMAT_ARGB32, "Unsupported pixel format");

            // In theory we shouldn’t rely on this.
            // In practice cairo picks a stride that is `width * size_of_pixel`
            // rounded up to 32 bits.
            // ARGB32 and RGB24 both use 32 bit per pixel, so rounding is a no-op.
            assert!(stride == width * (mem::size_of::<u32>() as i32),
                    "Expected 32bit pixel to make width satisfy stride requirements");

            assert!((data as usize) % mem::size_of::<u32>() == 0,
                    "Expected cairo to allocated data aligned to 32 bits");

            // FIXME: checked conversions
            Argb32Image {
                width: width as usize,
                height: height as usize,
                pixels: slice::from_raw_parts_mut(data as *mut u32, (width * height) as usize)
            }
        }
    }

    pub fn write_to_png_file<P: AsRef<path::Path>>(&self, filename: P) -> Result<(), Error> {
        self.write_to_png(io::BufWriter::new(fs::File::create(filename)?))
    }
}

// Private
struct CairoContext {
    ptr: *mut cairo_t,
}

impl CairoContext {
    fn check_status(&self) -> Result<(), CairoError> {
        CairoError::check(unsafe { cairo_status(self.ptr) })
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
    pub fn read_from_png<R: Read>(stream: R) -> Result<Self, Error> {
        let surface = with_c_callback! {
            stream: R: Read;
            fn callback(buffer: *mut c_uchar, length: c_uint) -> CAIRO_STATUS_WRITE_ERROR {
                // FIXME: checked conversion
                let slice = slice::from_raw_parts_mut(buffer, length as usize);
                stream.read_exact(slice)
            }
            (|ptr| ImageSurface { ptr })(cairo_image_surface_create_from_png_stream())
        };

        surface.check_status()?;
        Ok(surface)
    }

    pub fn write_to_png<W: Write>(&self, stream: W) -> Result<(), Error> {
        let status = with_c_callback! {
            stream: W: Write;
            fn callback(buffer: *const c_uchar, length: c_uint) -> CAIRO_STATUS_READ_ERROR {
                // FIXME: checked conversion
                let slice = slice::from_raw_parts(buffer, length as usize);
                stream.write_all(slice)
            }
            (|s| s)(cairo_surface_write_to_png_stream(self.ptr,))
        };

        CairoError::check(status)?;
        Ok(())
    }
}

macro_rules! c_error_impls {
    ($T: ty = |$self_: ident| $get_c_str_ptr: expr) => {
        impl StdError for $T {
            fn description(&self) -> &str {
                let cstr = unsafe {
                    let $self_ = self;
                    CStr::from_ptr($get_c_str_ptr)
                };
                cstr.to_str().unwrap()
            }
        }

        impl fmt::Display for $T {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(self.description())
            }
        }

        impl fmt::Debug for $T {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(self.description())
            }
        }
    }
}

#[derive(Clone)]
pub struct CairoError {
    status: cairo_status_t,
}

impl CairoError {
    fn check(status: cairo_status_t) -> Result<(), Self> {
        if status == CAIRO_STATUS_SUCCESS {
            Ok(())
        } else {
            Err(CairoError { status })
        }
    }
}

c_error_impls! {
    CairoError = |self_| cairo_status_to_string(self_.status)
}

pub struct GlibError {
    ptr: *mut GError,
}

impl Drop for GlibError {
    fn drop(&mut self) {
        unsafe {
            g_error_free(self.ptr)
        }
    }
}

c_error_impls! {
    GlibError = |self_| (*self_.ptr).message
}

macro_rules! error_enum {
    ($( $Variant: ident ($Type: ty), )+) => {
        #[derive(Debug)]
        pub enum Error {
            $(
                $Variant($Type),
            )+
        }

        $(
            impl From<$Type> for Error {
                fn from(e: $Type) -> Self {
                    Error::$Variant(e)
                }
            }
        )+
    }
}

error_enum! {
    Io(io::Error),
    Cairo(CairoError),
    Glib(GlibError),
}
