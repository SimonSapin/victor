#[macro_use]
extern crate lester;

use lester::{PdfDocument, RenderOptions};
use std::error::Error;

#[test]
fn zero_bytes_pdf() {
    match PdfDocument::from_bytes(b"") {
        Err(ref err)
            if err.description() == "PDF document is damaged"
                || err.description() == "Failed to load document" => {}
        Err(err) => panic!("expected 'damaged document' error, got {:?}", err),
        Ok(_) => panic!("expected error"),
    }
}

macro_rules! assert_approx_eq {
    ($a: expr, $b: expr) => {{
        let a = ($a * 1000.).round() / 1000.;
        let b = ($b * 1000.).round() / 1000.;
        assert_eq!(a, b)
    }};
}

#[test]
fn blank_pdf() {
    static PDF_BYTES: &[u8] = include_bytes!("A4_one_empty_page.pdf");
    let doc = PdfDocument::from_bytes(PDF_BYTES).unwrap();
    assert_eq!(doc.pages().len(), 1);
    assert!(doc.pages().nth(1).is_none());
    assert!(doc.pages().nth(2).is_none());
    let page = doc.pages().nth(0).unwrap();
    let (width, height) = page.size_in_ps_points();
    assert_approx_eq!(width, millimeters_to_poscript_points(210.));
    assert_approx_eq!(height, millimeters_to_poscript_points(297.));
}

fn millimeters_to_poscript_points(mm: f64) -> f64 {
    let inches = mm / 25.4;
    inches * 72.
}

#[test]
fn pattern_4x4_pdf() {
    static PDF_BYTES: &[u8] = include_bytes!("pattern_4x4.pdf");
    let doc = PdfDocument::from_bytes(PDF_BYTES).unwrap();
    let page = doc.pages().next().unwrap();
    assert_eq!(page.size_in_ps_points(), (3., 3.));
    assert_eq!(page.size_in_css_px(), (4., 4.));

    let options = RenderOptions {
        // Apparently this (in addition to having `/Interpolate false` in the PDF file)
        // is required to convince Poppler to use use nearest-neighbor instead of bilinear
        // when "interpolating" a 4x4 image source into a 4x4 surface.
        for_printing: true,

        ..RenderOptions::default()
    };
    let mut surface = page.render_with_options(options).unwrap();
    const RED: u32 = 0xFFFF_0000;
    const BLUE: u32 = 0xFF00_00FF;
    assert_pixels_eq!(
        surface.pixels().buffer,
        &[
            RED, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
            BLUE, BLUE,
        ]
    );

    let mut surface = page
        .render_with_options(RenderOptions {
            dppx_x: 2.0,
            dppx_y: 3.0,
            ..RenderOptions::default()
        })
        .unwrap();
    let pixels = surface.pixels();
    assert_eq!((pixels.width, pixels.height), (8, 12));
}
