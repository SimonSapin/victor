extern crate lester;

#[test]
fn version() {
    assert!(lester::poppler_version().starts_with("0."))
}
