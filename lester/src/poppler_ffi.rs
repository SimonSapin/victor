#![allow(non_camel_case_types)]

use cairo_ffi::cairo_t;
use std::os::raw::*;

extern "C" {
    pub fn poppler_document_new_from_data(data: *mut c_char,
                                          length: c_int,
                                          password: *const c_char,
                                          error: *mut *mut GError)
                                          -> *mut PopplerDocument;
    pub fn poppler_document_get_n_pages(document: *mut PopplerDocument) -> c_int;
    pub fn poppler_document_get_page(document: *mut PopplerDocument, index: c_int) -> *mut PopplerPage;

    pub fn poppler_page_get_size(page: *mut PopplerPage, width: *mut f64, height: *mut f64);
    pub fn poppler_page_render(page: *mut PopplerPage, cairo: *mut cairo_t);
    pub fn poppler_page_render_for_printing(page: *mut PopplerPage, cairo: *mut cairo_t);

    pub fn g_error_free(error: *mut GError);
    pub fn g_object_unref(object: gpointer);
}

pub type gpointer = *mut c_void;
pub type gchar = c_char;
pub type gint = c_int;
pub type guint32 = c_uint;
pub type GQuark = guint32;

#[repr(C)]
pub struct PopplerDocument { opaque: [u8; 0] }

#[repr(C)]
pub struct PopplerPage { opaque: [u8; 0] }

#[repr(C)]
pub struct GError {
    pub domain: GQuark,
    pub code: gint,
    pub message: *mut gchar,
}
