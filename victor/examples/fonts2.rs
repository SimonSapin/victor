extern crate victor;

use victor::fonts2::Font;

fn main() {
    let f: Font = Font::parse(victor::fonts::BITSTREAM_VERA_SANS.bytes()).unwrap();
    println!("{:#?}", f);
}
