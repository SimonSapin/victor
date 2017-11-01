extern crate lester;

use lester::{PdfDocument, ImageSurface};
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
    assert_eq!(doc.page_count(), 1);
    assert!(doc.get_page(1).is_none());
    assert!(doc.get_page(2).is_none());
    let page = doc.get_page(0).unwrap();
    let (width, height) = page.size();
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
    assert_eq!(doc.page_count(), 1);
    let page = doc.get_page(0).unwrap();
    assert_eq!(page.size(), (3., 3.));  // 4px == 3pt

    let mut surface = ImageSurface::new_rgb24(4, 4).unwrap();
    page.render(&mut surface).unwrap();
}
