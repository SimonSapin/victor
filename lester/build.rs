extern crate pkg_config;

fn main() {
    // FIXME: Do we actually require a more recent version than this?
    pkg_config::Config::new().atleast_version("1.0.0").probe("cairo").unwrap();
}
