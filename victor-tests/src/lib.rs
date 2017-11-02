extern crate lester;
extern crate victor;

use victor::display_lists::{Document, Page};

#[test]
fn pdf() {
    let dl = Document {
        pages: vec![
            Page {
                width_in_ps_points: 100.,
                height_in_ps_points: 200.,
            },
            Page {
                width_in_ps_points: 300.,
                height_in_ps_points: 400.,
            },
        ],
    };
    let bytes = dl.write_to_pdf_bytes().unwrap();
    let _doc = lester::PdfDocument::from_bytes(&bytes);
}
