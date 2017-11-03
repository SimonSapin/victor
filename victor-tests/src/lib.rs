#[macro_use] extern crate lester;
extern crate victor;

use victor::display_lists::*;
use victor::euclid::rect;

#[test]
fn pdf() {
    let dl = Document {
        pages: vec![
            Page {
                size: Size::new(100., 200.),
                display_items: vec![],
            },
            Page {
                size: Size::new(4., 4.),
                display_items: vec![
                    DisplayItem::SolidRectangle(rect(0., 0., 4., 4.), RGB(0., 0., 1.)),
                    DisplayItem::SolidRectangle(rect(0., 0., 1., 1.), RGB(1., 0., 0.)),
                ],
            },
        ],
    };
    let bytes = dl.write_to_pdf_bytes().unwrap();
    println!("{}", String::from_utf8_lossy(&bytes));
    let doc = lester::PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(doc.producer().unwrap().to_str().unwrap(),
               "Victor <https://github.com/SimonSapin/victor>");

    let sizes: Vec<_> = doc.pages().map(|page| page.size_in_ps_points()).collect();
    assert_eq!(sizes, [(75., 150.), (3., 3.)]);

    let mut surface = doc.pages().nth(1).unwrap().render_with_default_options().unwrap();
    const RED: u32 = 0xFFFF_0000;
    const BLUE: u32 = 0xFF00_00FF;
    assert_pixels_eq!(surface.pixels().buffer, &[
        RED,  BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
    ]);
}
