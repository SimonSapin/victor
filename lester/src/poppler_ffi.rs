#![allow(non_camel_case_types)]

use std::os::raw::*;

extern "C" {
    pub fn poppler_get_version() -> *const c_char;
}
