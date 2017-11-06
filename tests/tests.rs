#![cfg(test)]

#[macro_use] extern crate lester;
extern crate victor;

use lester::{PdfDocument, RenderOptions};
use std::env;
use std::fs::File;
use std::io::Write;
use victor::display_lists::*;
use victor::fonts::Font;

#[test]
fn pdf() {
    let vera = include_bytes!("../fonts/ttf-bitstream-vera-1.10/Vera.ttf");
    let vera = Font::from_bytes(&vera[..]).unwrap();
    let dl = Document {
        pages: vec![
            Page {
                size: Size::new(120., 50.),
                display_items: vec![
                    DisplayItem::Text {
                        glyph_ids: vera.to_glyph_ids("Têst→iimm"),
                        font: vera,
                        font_size: Length::new(20.),
                        color: RGBA(0., 0., 0., 1.),
                        start: point(10., 30.),
                    }
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
    assert_eq!(sizes, [(120., 50.), (4., 4.)]);

    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        doc.pages().nth(0).unwrap()
            .render_with_default_options().unwrap()
            .write_to_png_file("/tmp/victor.png").unwrap()
    }
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
