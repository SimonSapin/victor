extern crate lester;

use lester::PdfDocument;
use std::error::Error;

#[test]
fn empty_pdf() {
    match PdfDocument::from_bytes(b"") {
        Err(ref err) if err.description() == "PDF document is damaged" => {}
        Err(err) => panic!("expected 'damaged document' error, got {:?}", err),
        Ok(_) => panic!("expected error")
    }
}
