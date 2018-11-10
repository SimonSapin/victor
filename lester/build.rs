fn main() {
    // We rely on poppler-glib’s dependency on cairo to link cairo.
    // If we make a second pkg-config call, rustc complains with
    // "warning: redundant linker flag specified for library `cairo`"

    pkg_config::Config::new()
        // FIXME: Do we actually require a more recent version than this?
        .atleast_version("0.16.0")
        .probe("poppler-glib")
        .unwrap();
}
