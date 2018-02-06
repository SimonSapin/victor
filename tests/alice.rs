use std::env;
use std::fs::File;
use std::io::Write;
use victor::text::*;
use victor::text_plain;
use victor::fonts::BITSTREAM_VERA_SANS;
use victor::primitives::*;

static ALICE: &'static str = include_str!("alice.txt");

#[test]
fn breaks() {
    let words = split_at_breaks(ALICE);
    assert_eq!(&words[..10], &["CHAPTER ", "II. ", "The ", "Pool ", "of ", "Tears\n", "\n",
                              "‘Curiouser ", "and ", "curiouser!’ "]);
}

#[test]
fn hard_breaks() {
    let lines = split_at_hard_breaks(ALICE);
    assert_eq!(lines[0], "CHAPTER II. The Pool of Tears\n");
    assert_eq!(lines[1], "\n");
    assert!(lines.last().unwrap().ends_with("I am so VERY tired of being all alone here!’\n"))
}

#[test]
fn render() {
    let style = text_plain::Style {
        page_size: Size::new(210., 297.),
        page_margin: Length::new(20.),
        font: BITSTREAM_VERA_SANS.get().unwrap(),
        font_size: Length::new(16.),
        line_height: 1.2,
    };
    let pdf_bytes = text_plain::layout(ALICE, &style).unwrap().write_to_pdf_bytes();

    if env::var("VICTOR_WRITE_TO_TMP").is_ok() {
        File::create("/tmp/alice.pdf").unwrap().write_all(&pdf_bytes).unwrap();
    }
    assert!(pdf_bytes == include_bytes!("alice.pdf").as_ref());
}
