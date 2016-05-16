#[macro_use] extern crate string_cache;
extern crate victor;

use std::path::{Path, PathBuf};
use std::env;

fn main() {
    let filename = match env::args().nth(1) {
        Some(arg) => PathBuf::from(arg),
        None => Path::new(file!()).parent().unwrap().join("svg").join("rust-logo-blk.svg")
    };
    let doc = match victor::xml::Node::parse_file(filename) {
        Ok(doc) => doc,
        Err(error) => {
            println!("{:?}", error);
            return
        }
    };
    let selector = victor::SelectorList::parse("path[d]").unwrap();
    let mut cursor = doc.cursor();
    while selector.query_next(&mut cursor) {
        let element = cursor.element();
        println!("<path>");
        let attribute = element.attribute(&atom!("d")).unwrap();
        let mut path = victor::svg::path::parse(attribute).simplify();
        let mut current_point = None;
        for command in &mut path {
            use victor::svg::path::SimpleCommand::*;

            println!("    {:?}", command);
            match command {
                Move { to } | Line { to } | Curve { to, .. } => current_point = Some(to),
                ClosePath => {}
                EllipticalArc(arc) => {
                    let approximation = arc.to_cubic_bezier(current_point.unwrap());
                    for approximation_command in &approximation {
                        println!("        {:?}", approximation_command)
                    }
                    current_point = Some(arc.to);
                }
            }
        }
        if let Some(error) = path.error() {
            println!("");
            println!("    Error around byte {}: {}.", error.position, error.reason);
        }

        // FIXME: cursor apparently needs a state where it doesn’t point to an element?
        if !cursor.next_in_tree_order() {
            break
        }
    }
}
