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
    println!("{}", String::from_utf8_lossy(&bytes));
    let doc = lester::PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(doc.producer().unwrap().to_str().unwrap(),
               "Victor <https://github.com/SimonSapin/victor>");
    let sizes: Vec<_> = doc.pages().map(|page| page.size_in_ps_points()).collect();
    assert_eq!(sizes, [(100., 200.), (300., 400.)])
}
