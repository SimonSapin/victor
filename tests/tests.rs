#![cfg(test)]

#[macro_use] extern crate lester;
#[macro_use] extern crate victor;

use lester::{PdfDocument, RenderOptions, Backdrop};
use std::env;
use std::fs::File;
use std::io::Write;
use victor::display_lists::*;
use victor::fonts::{BITSTREAM_VERA_SANS, LazyStaticFont};

static AHEM: LazyStaticFont = include_font!("fonts/ahem/ahem.ttf");
static NOTO: LazyStaticFont = include_font!("fonts/noto/NotoSansLinearB-Regular.ttf");

#[test]
fn pdf() {
    let vera = BITSTREAM_VERA_SANS.get().unwrap();
    let noto = NOTO.get().unwrap();
    let ahem = AHEM.get().unwrap();
    let dl = Document {
        pages: vec![
            Page {
                size: Size::new(140., 50.),
                display_items: vec![
                    DisplayItem::Text {
                        glyph_ids: vera.to_glyph_ids("Têst→iimm"),
                        font: vera,
                        font_size: Length::new(15.),
                        color: RGBA(0., 0., 0., 1.),
                        start: point(10., 20.),
                    },
                    DisplayItem::Text {
                        glyph_ids: ahem.to_glyph_ids("pÉX"),
                        font: ahem,
                        font_size: Length::new(15.),
                        color: RGBA(0., 0., 0., 1.),
                        start: point(10., 40.),
                    },
                    DisplayItem::Text {
                        glyph_ids: noto.to_glyph_ids("𐁉 𐁁𐀓𐀠𐀴𐀍"),
                        font: noto,
                        font_size: Length::new(15.),
                        color: RGBA(0., 0., 0., 1.),
                        start: point(65., 40.),
                    },
                ],
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
    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        File::create("/tmp/victor.pdf").unwrap().write_all(&bytes).unwrap();
    }
    if env::var("VICTOR_PRINT").is_ok() {
        println!("{}", String::from_utf8_lossy(&bytes));
    }
    let doc = PdfDocument::from_bytes(&bytes).unwrap();
    assert_eq!(doc.producer().unwrap().to_str().unwrap(),
               "Victor <https://github.com/SimonSapin/victor>");

    let sizes: Vec<_> = doc.pages().map(|page| page.size_in_css_px()).collect();
    assert_eq!(sizes, [(140., 50.), (4., 4.)]);

    // FIXME: find a way to round-trip code points without a glyph like '→'
    assert_eq!(doc.pages().nth(0).unwrap().text().unwrap().to_str().unwrap(),
               "Têst iimm\npÉX 𐁉 𐁁𐀓𐀠𐀴𐀍");

    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        doc.pages()
           .nth(0).unwrap()
           .render().unwrap()
           .write_to_png_file("/tmp/victor.png").unwrap()
    }
    let page = doc.pages().nth(1).unwrap();
    let mut surface = page.render().unwrap();
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

    let mut surface = page.render_with_options(RenderOptions {
        dppx_x: 2.0,
        dppx_y: 3.0,
        backdrop: Backdrop::White,
        ..RenderOptions::default()
    }).unwrap();
    let pixels = surface.pixels();
    assert_eq!((pixels.width, pixels.height), (8, 12));
    {
        const RED_: u32 = 0xFFFF_7F7F;
        const ____: u32 = 0xFFFF_FFFF;
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
}
