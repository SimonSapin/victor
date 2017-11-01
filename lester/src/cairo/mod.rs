use Argb32Image;
use self::ffi::*;
use std::error::Error as StdError;
use std::ffi::CStr;
use std::fmt;
use std::io::{self, Read, Write};
use std::mem;
use std::os::raw::*;
use std::slice;

mod ffi;  // Not public or re-exported

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
    fn check_status(&self) -> Result<(), Error> {
        Error::check(unsafe { cairo_surface_status(self.ptr) })
    }

    pub fn as_image(&mut self) -> Argb32Image {
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

    pub fn read_from_png<R: Read>(stream: R) -> Result<Self, ::Error> {
        struct ClosureData<R> {
            stream: R,
            status: Result<(), io::Error>,
        };
        let mut closure_data = ClosureData {
            stream,
            status: Ok(()),
        };
        let closure_ptr: *mut ClosureData<R> = &mut closure_data;

        unsafe extern "C" fn read_callback<R: Read>(
            closure_ptr: *mut c_void, buffer: *mut c_uchar, length: c_uint,
        ) -> cairo_status_t {
            // FIXME: catch panics

            let closure_data = &mut *(closure_ptr as *mut ClosureData<R>);
            if closure_data.status.is_err() {
                return CAIRO_STATUS_READ_ERROR
            }

            // FIXME: checked conversion
            let slice = slice::from_raw_parts_mut(buffer, length as usize);

            match closure_data.stream.read_exact(slice) {
                Ok(()) => {
                    CAIRO_STATUS_SUCCESS
                }
                Err(error) => {
                    closure_data.status = Err(error);
                    CAIRO_STATUS_READ_ERROR
                }
            }
        }

        let ptr = unsafe {
            cairo_image_surface_create_from_png_stream(read_callback::<R>, closure_ptr as *mut c_void)
        };
        let surface = ImageSurface { ptr };
        closure_data.status?;
        surface.check_status()?;
        Ok(surface)
    }

    pub fn write_to_png<W: Write>(&self, stream: W) -> Result<(), ::Error> {
        struct ClosureData<W> {
            stream: W,
            status: Result<(), io::Error>,
        };
        let mut closure_data = ClosureData {
            stream,
            status: Ok(()),
        };
        let closure_ptr: *mut ClosureData<W> = &mut closure_data;

        unsafe extern "C" fn write_callback<W: Write>(
            closure_ptr: *mut c_void, buffer: *const c_uchar, length: c_uint,
        ) -> cairo_status_t {
            // FIXME: catch panics

            let closure_data = &mut *(closure_ptr as *mut ClosureData<W>);
            if closure_data.status.is_err() {
                return CAIRO_STATUS_READ_ERROR
            }

            // FIXME: checked conversion
            let slice = slice::from_raw_parts(buffer, length as usize);

            match closure_data.stream.write_all(slice) {
                Ok(()) => {
                    CAIRO_STATUS_SUCCESS
                }
                Err(error) => {
                    closure_data.status = Err(error);
                    CAIRO_STATUS_READ_ERROR
                }
            }
        }

        let status = unsafe {
            cairo_surface_write_to_png_stream(
                self.ptr,
                write_callback::<W>,
                closure_ptr as *mut c_void
            )
        };
        Error::check(status)?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Error {
    status: cairo_status_t,
}

impl Error {
    fn check(status: cairo_status_t) -> Result<(), Error> {
        if status == CAIRO_STATUS_SUCCESS {
            Ok(())
        } else {
            Err(Error { status })
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        let cstr = unsafe {
            CStr::from_ptr(cairo_status_to_string(self.status))
        };
        cstr.to_str().unwrap()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}
