use std::os::raw::c_int;

extern "C" {
    pub fn cairo_version() -> c_int;
}

#[test]
fn it_works() {
    unsafe {
        assert!(cairo_version() > 1_12_00);
        assert!(cairo_version() < 1_17_00);
    }
}
