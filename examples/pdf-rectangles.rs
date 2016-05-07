extern crate victor;

use std::fs::File;
use std::io::BufWriter;
use victor::pdf::document_structure::{render, Rect, Color};

fn main() {
    render(
        &[
            (Rect { x: 100., y: 100., width: 200., height: 200. }, Color { r: 0, g: 0xFF, b: 0}),
            (Rect { x: 200., y: 150., width: 150., height: 250. }, Color { r: 0, g: 0, b: 0xFF}),
        ],
        Rect { x: 0., y: 0., width: 800., height: 600. },
        BufWriter::new(File::create("out.pdf").unwrap())
    ).unwrap();
}
