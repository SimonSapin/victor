use std::env;
use std::fs::File;
use std::io::Write;
use victor::fonts::BITSTREAM_VERA_SANS;
use victor::primitives::*;
use victor::text_plain;

static ALICE: &'static str = include_str!("alice.txt");

#[test]
fn render() {
    let style = text_plain::Style {
        page_size: Size::new(210., 297.),
        page_margin: Length::new(20.),
        font: BITSTREAM_VERA_SANS.clone(),
        font_size: Length::new(16.),
        line_height: 1.5,
        justify: true,
    };
    let pdf_bytes = text_plain::layout(ALICE, &style)
        .unwrap()
        .write_to_pdf_bytes();

    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        File::create("/tmp/alice.pdf")
            .unwrap()
            .write_all(&pdf_bytes)
            .unwrap();
    }
    assert!(pdf_bytes == include_bytes!("alice.pdf").as_ref());
}
