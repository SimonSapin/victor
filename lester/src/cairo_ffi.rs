#![allow(non_camel_case_types)]

use std::os::raw::*;

extern "C" {
    pub fn cairo_surface_destroy(surface: *mut cairo_surface_t);
    pub fn cairo_surface_status(surface: *mut cairo_surface_t) -> cairo_status_t;
    pub fn cairo_image_surface_create(
        format: cairo_format_t,
        width: c_int,
        height: c_int,
    ) -> *mut cairo_surface_t;
    pub fn cairo_image_surface_get_width(surface: *mut cairo_surface_t) -> c_int;
    pub fn cairo_image_surface_get_height(surface: *mut cairo_surface_t) -> c_int;
    pub fn cairo_image_surface_get_stride(surface: *mut cairo_surface_t) -> c_int;
    pub fn cairo_image_surface_get_data(surface: *mut cairo_surface_t) -> *mut c_uchar;
    pub fn cairo_image_surface_get_format(surface: *mut cairo_surface_t) -> cairo_format_t;
    pub fn cairo_image_surface_create_from_png_stream(
        read_func: cairo_read_func_t,
        closure: *mut c_void,
    ) -> *mut cairo_surface_t;
    pub fn cairo_surface_write_to_png_stream(
        surface: *mut cairo_surface_t,
        write_func: cairo_write_func_t,
        closure: *mut c_void,
    ) -> cairo_status_t;
    pub fn cairo_surface_flush(surface: *mut cairo_surface_t);

    pub fn cairo_create(target: *mut cairo_surface_t) -> *mut cairo_t;
    pub fn cairo_set_source_rgb(cr: *mut cairo_t, red: f64, green: f64, blue: f64);
    pub fn cairo_paint(cr: *mut cairo_t);
    pub fn cairo_scale(cr: *mut cairo_t, sx: f64, sy: f64);
    pub fn cairo_set_antialias(cr: *mut cairo_t, antialias: cairo_antialias_t);
    pub fn cairo_status(cr: *mut cairo_t) -> cairo_status_t;
    pub fn cairo_destroy(cr: *mut cairo_t);

    pub fn cairo_status_to_string(status: cairo_status_t) -> *const c_char;
}

pub type cairo_status_t = c_uint;
pub type cairo_format_t = c_int;
pub type cairo_antialias_t = c_uint;

pub const CAIRO_STATUS_SUCCESS: cairo_status_t = 0;
pub const CAIRO_STATUS_READ_ERROR: cairo_status_t = 10;
pub const CAIRO_STATUS_WRITE_ERROR: cairo_status_t = 11;

pub const CAIRO_ANTIALIAS_DEFAULT: cairo_antialias_t = 0;
pub const CAIRO_ANTIALIAS_NONE: cairo_antialias_t = 1;
pub const CAIRO_ANTIALIAS_GRAY: cairo_antialias_t = 2;
pub const CAIRO_ANTIALIAS_SUBPIXEL: cairo_antialias_t = 3;
pub const CAIRO_ANTIALIAS_FAST: cairo_antialias_t = 4;
pub const CAIRO_ANTIALIAS_GOOD: cairo_antialias_t = 5;
pub const CAIRO_ANTIALIAS_BEST: cairo_antialias_t = 6;

pub const CAIRO_FORMAT_ARGB32: cairo_format_t = 0;
pub const CAIRO_FORMAT_RGB24: cairo_format_t = 1;

pub type cairo_read_func_t = unsafe extern "C" fn(
    closure: *mut c_void,
    data: *mut c_uchar,
    length: c_uint,
) -> cairo_status_t;

pub type cairo_write_func_t = unsafe extern "C" fn(
    closure: *mut c_void,
    data: *const c_uchar,
    length: c_uint,
) -> cairo_status_t;

#[repr(C)]
pub struct cairo_surface_t {
    opaque: [u8; 0],
}

#[repr(C)]
pub struct cairo_t {
    opaque: [u8; 0],
}
