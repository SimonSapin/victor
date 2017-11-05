#[macro_use] extern crate lester;
extern crate victor;

use lester::{PdfDocument, RenderOptions};
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
    let doc = PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(doc.producer().unwrap().to_str().unwrap(),
               "Victor <https://github.com/SimonSapin/victor>");

    let sizes: Vec<_> = doc.pages().map(|page| page.size_in_css_px()).collect();
    assert_eq!(sizes, [(20., 10.), (4., 4.)]);

    let page = doc.pages().nth(1).unwrap();
    let mut surface = page.render_with_default_options().unwrap();
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

    let mut surface = page.render(RenderOptions {
        dppx_x: 2.0,
        dppx_y: 3.0,
        ..RenderOptions::default()
    }).unwrap();
    let pixels = surface.pixels();
    assert_eq!((pixels.width, pixels.height), (8, 12));
    assert_pixels_eq!(pixels.buffer, &[
        RED_, RED_, ____, ____, ____, ____, ____, ____,
        RED_, RED_, ____, ____, ____, ____, ____, ____,
        RED_, RED_, ____, ____, ____, ____, ____, ____,
        BOTH, BOTH, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BOTH, BOTH, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BOTH, BOTH, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
        BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE, BLUE,
    ][..]);
}
