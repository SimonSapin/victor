extern crate lester;

use lester::PdfDocument;
use std::error::Error;

#[test]
fn zero_bytes_pdf() {
    match PdfDocument::from_bytes(b"") {
        Err(ref err) if err.description() == "PDF document is damaged" => {}
        Err(err) => panic!("expected 'damaged document' error, got {:?}", err),
        Ok(_) => panic!("expected error")
    }
}

#[test]
fn blank_pdf() {
    static PDF_BYTES: &[u8] = include_bytes!("A4_one_empty_page.pdf");
    let doc = PdfDocument::from_bytes(PDF_BYTES).unwrap();
}
