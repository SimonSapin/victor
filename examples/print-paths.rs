#[macro_use] extern crate string_cache;
extern crate victor;

use std::path::{Path, PathBuf};
use std::env;

fn main() {
    let filename = match env::args().nth(1) {
        Some(arg) => PathBuf::from(arg),
        None => Path::new(file!()).parent().unwrap().join("rust-logo").join("rust-logo-blk.svg")
    };
    let parser = victor::xml::Parser::new();
    let doc = match parser.parse_file(filename) {
        Ok(doc) => doc,
        Err(error) => {
            println!("{:?}", error);
            return
        }
    };
    let selector = victor::SelectorList::parse("path[d]").unwrap();
    doc.iter(&mut |node| {
        if let Some(element) = node.as_element() {
            if selector.matches(element) {
                println!("<path>");
                let attribute = element.attribute(&atom!("d")).unwrap();
                let mut path = victor::svg::path::parse(attribute);
                for command in &mut path {
                    println!("    {:?}", command)
                }
                if let Some(error) = path.error() {
                    println!("");
                    println!("    Error around byte {}: {}.", error.position, error.reason);
                }
            }
        }
    })
}
