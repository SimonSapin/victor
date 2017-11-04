#[macro_use] extern crate lester;
extern crate victor;

use victor::display_lists::*;
use victor::euclid::rect;

#[test]
fn pdf() {
    let dl = Document {
        pages: vec![
            Page {
                size: Size::new(20., 10.),
                display_items: vec![],
            },
            Page {
                size: Size::new(4., 4.),
                display_items: vec![
                    DisplayItem::SolidRectangle(rect(0., 1., 4., 3.), RGBA(0., 0., 1., 1.)),
                    DisplayItem::SolidRectangle(rect(0., 0., 1., 2.), RGBA(1., 0., 0., 0.5)),
                ],
            },
        ],
    };
    let bytes = dl.write_to_pdf_bytes();
    println!("{}", String::from_utf8_lossy(&bytes));
    let doc = lester::PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(doc.producer().unwrap().to_str().unwrap(),
               "Victor <https://github.com/SimonSapin/victor>");

    let sizes: Vec<_> = doc.pages().map(|page| page.size_in_ps_points()).collect();
    assert_eq!(sizes, [(15., 7.5), (3., 3.)]);

    let mut surface = doc.pages().nth(1).unwrap().render_with_default_options().unwrap();
    const RED_: u32 = 0x8080_0000;
    const BLUE: u32 = 0xFF00_00FF;
    const BOTH: u32 = 0xFF80_007F;
    const ____: u32 = 0x0000_0000;
    assert_pixels_eq!(surface.pixels().buffer, &[
        RED_, ____, ____, ____,
        BOTH, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE,
    ]);
}
