extern crate lester;

use lester::{PdfDocument, RenderOptions};
use std::error::Error;

#[test]
fn zero_bytes_pdf() {
    match PdfDocument::from_bytes(b"") {
        Err(ref err) if err.description() == "PDF document is damaged" => {}
        Err(err) => panic!("expected 'damaged document' error, got {:?}", err),
        Ok(_) => panic!("expected error")
    }
}

macro_rules! assert_approx_eq {
    ($a: expr, $b: expr) => {
        {
            let a = ($a * 1000.).round() / 1000.;
            let b = ($b * 1000.).round() / 1000.;
            assert_eq!(a, b)
        }
    }
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

macro_rules! assert_pixels_eq {
    ($a: expr, $b: expr) => {
        {
            let a = $a;
            let b = $b;
            if a != b {
                panic!("{} != {}\n[{}]\n[{}]", stringify!($a), stringify!($b),
                       hex(a), hex(b))
            }
        }
    }
}

fn hex(pixels: &[u32]) -> String {
    pixels.iter().map(|p| format!("{:08X}", p)).collect::<Vec<_>>().join(", ")
}

#[test]
fn pattern_4x4_pdf() {
    static PDF_BYTES: &[u8] = include_bytes!("pattern_4x4.pdf");
    let doc = PdfDocument::from_bytes(PDF_BYTES).unwrap();
    let page = doc.pages().next().unwrap();
    assert_eq!(page.size_in_ps_points(), (3., 3.));  // 4px == 3pt

    let options = RenderOptions {
        for_printing: true,
        ..RenderOptions::default()
    };
    let mut surface = page.render(options).unwrap();
    const RED: u32 = 0x00FF_0000;
    const BLUE: u32 = 0x0000_00FF;
    assert_pixels_eq!(surface.pixels().buffer, &[
        RED,  BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
    ]);
}
