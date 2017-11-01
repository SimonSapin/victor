#![allow(non_camel_case_types)]

use std::os::raw::*;

extern "C" {
    pub fn cairo_surface_destroy(surface: *mut cairo_surface_t);
    pub fn cairo_surface_status(surface: *mut cairo_surface_t) -> cairo_status_t;
    pub fn cairo_image_surface_get_width(surface: *mut cairo_surface_t) -> c_int;
    pub fn cairo_image_surface_get_height(surface: *mut cairo_surface_t) -> c_int;
    pub fn cairo_image_surface_get_stride(surface: *mut cairo_surface_t) -> c_int;
    pub fn cairo_image_surface_get_data(surface: *mut cairo_surface_t) -> *mut c_uchar;
    pub fn cairo_image_surface_get_format(surface: *mut cairo_surface_t) -> cairo_format_t;
    pub fn cairo_image_surface_create_from_png_stream(
        read_func: cairo_read_func_t,
        closure: *mut c_void
    ) -> *mut cairo_surface_t;

    pub fn cairo_status_to_string(status: cairo_status_t) -> *const c_char;
}

pub type cairo_status_t = c_uint;
pub type cairo_format_t = c_int;

pub const CAIRO_STATUS_SUCCESS: cairo_status_t = 0;
pub const CAIRO_STATUS_READ_ERROR: cairo_status_t = 10;

pub const CAIRO_FORMAT_ARGB32: cairo_format_t = 0;
pub const CAIRO_FORMAT_RGB24: cairo_format_t = 1;

pub type cairo_read_func_t = unsafe extern "C" fn(closure: *mut c_void, data: *mut c_uchar,
                                                  length: c_uint) -> cairo_status_t;

#[repr(C)]
pub struct cairo_surface_t { opaque: [u8; 0] }