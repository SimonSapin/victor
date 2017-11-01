#![allow(non_camel_case_types)]

use std::os::raw::*;

extern "C" {
    pub fn g_error_free(error: *mut GError);
}

pub type gchar = c_char;
pub type gint = c_int;
pub type guint32 = c_uint;
pub type GQuark = guint32;

#[repr(C)]
pub struct GError {
    pub domain: GQuark,
    pub code: gint,
    pub message: *mut gchar,
}
