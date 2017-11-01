extern crate lester;

use lester::CairoImageSurface;
use std::io;

#[test]
fn empty_png_fails() {
    match CairoImageSurface::read_from_png("".as_bytes()) {
        Err(lester::Error::Io(err)) => {
            match err.kind() {
                io::ErrorKind::UnexpectedEof => {}
                _ => panic!("Expected an UnexpectedEof error, got {:?}", err)
            }
        }
        Err(err) => panic!("Expected an IO error, got {:?}", err),
        _ => panic!("Expected an error"),
    }
}

#[test]
fn round_trip_png() {
    static PNG_BYTES: &[u8] = include_bytes!("pattern_4x4.png");
    let mut surface = CairoImageSurface::read_from_png(PNG_BYTES).unwrap();

    fn assert_expected_image(image: lester::Argb32Image) {
        assert_eq!(image.width, 4);
        assert_eq!(image.height, 4);
        // ARGB32
        const RED: u32 = 0xFFFF_0000;
        const BLUE: u32 = 0xFF00_00FF;
        assert_eq!(image.pixels, &[
            RED,  BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE, BLUE, BLUE,
        ]);
    }

    assert_expected_image(surface.as_image());

    let mut bytes = Vec::new();
    surface.write_to_png(&mut bytes).unwrap();

    let mut surface2 = CairoImageSurface::read_from_png(&*bytes).unwrap();
    assert_expected_image(surface2.as_image());
}
