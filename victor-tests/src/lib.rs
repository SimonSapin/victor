extern crate lester;
extern crate victor;

use victor::display_lists::{Document, Page, Size};

#[test]
fn pdf() {
    let dl = Document {
        pages: vec![
            Page {
                size: Size::new(100., 200.),
                display_items: vec![],
            },
            Page {
                size: Size::new(300., 400.),
                display_items: vec![],
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
