extern crate victor;

fn main() {
    victor::fonts2::parse(victor::fonts::BITSTREAM_VERA_SANS.bytes()).unwrap()
}
