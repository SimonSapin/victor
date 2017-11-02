use cairo_ffi::{cairo_status_t, cairo_status_to_string, CAIRO_STATUS_SUCCESS};
use std::error::Error;
use std::ffi::CStr;
use std::fmt;
use std::io;
use poppler_ffi::{GError, g_error_free};

macro_rules! c_error_impls {
    ($T: ty = |$self_: ident| $get_c_str_ptr: expr) => {
        impl Error for $T {
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

/// An error returned by cairo
#[derive(Clone)]
pub struct CairoError {
    status: cairo_status_t,
}

impl CairoError {
    pub(crate) fn check(status: cairo_status_t) -> Result<(), Self> {
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

/// A `glib` error returned by Poppler
pub struct GlibError {
    pub(crate) ptr: *mut GError,
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
        /// An error returned by Lester.
        #[derive(Debug)]
        pub enum LesterError {
            $(
                $Variant($Type),
            )+
        }

        $(
            impl From<$Type> for LesterError {
                fn from(e: $Type) -> Self {
                    LesterError::$Variant(e)
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
