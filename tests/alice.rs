use victor::text::*;
use victor::text_plain;
use victor::fonts::BITSTREAM_VERA_SANS;

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
    let vera = BITSTREAM_VERA_SANS.get().unwrap();
    let doc = text_plain::layout(ALICE, &vera).unwrap().write_to_pdf_bytes();
}
