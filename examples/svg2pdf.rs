extern crate cssparser;
#[macro_use] extern crate string_cache;
extern crate victor;

use cssparser::Color;
use cssparser::Parser as CssParser;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use victor::xml;
use victor::pdf::document_structure as pdf;

fn main() {
    render_doc().unwrap()
}

fn render_doc() -> xml::Result<()> {
    let filename = match std::env::args().nth(1) {
        Some(arg) => PathBuf::from(arg),
        None => Path::new(file!()).parent().unwrap().join("rust-logo").join("rust-logo-blk.svg")
    };
    let parser = xml::Parser::new();
    let doc = try!(parser.parse_file(filename));
    let mut pdf = try!(pdf::PdfDocument::create_file("out.pdf"));
    try!(pdf.write_page(144., 144., |page| {
        render_node(doc, page)
    }));
    try!(pdf.finish());
    Ok(())
}

fn render_node<W: Write>(node: xml::Ref, page: &mut pdf::Page<W>) -> io::Result<()> {
    if let Some(element) = node.as_element() {
        if let Some(fill) = element.attribute(&atom!("fill")) {
            match CssParser::new(fill).parse_entirely(Color::parse) {
                Ok(Color::RGBA(c)) => try!(page.non_stroking_color(c.red, c.green, c.blue)),
                Ok(Color::CurrentColor) | Err(()) => {}
            }
        }
        if element.data.name == qualname!(svg, "path") {
            if let Some(d_attribute) = element.attribute(&atom!("d")) {
                try!(render_path(d_attribute, page));
            }
        }
    }

    let mut link = node.first_child();
    while let Some(child) = link {
        try!(render_node(child, page));
        link = child.next_sibling()
    }
    Ok(())
}

fn render_path<W: Write>(d_attribute: &str, page: &mut pdf::Page<W>) -> io::Result<()> {
    let mut path = victor::svg::path::parse(d_attribute).simplify();
    let mut current_point = None;
    for command in &mut path {
        use victor::svg::path::SimpleCommand::*;

        match command {
            Move { to } => {
                try!(page.move_to(to));
                current_point = Some(to)
            }
            Line { to } => {
                try!(page.line_to(to));
                current_point = Some(to)
            }
            Curve { control_1, control_2, to } => {
                try!(page.curve_to(control_1, control_2, to));
                current_point = Some(to)
            }
            ClosePath => {
                try!(page.close_path())
            }
            EllipticalArc(arc) => {
                let approximation = arc.to_cubic_bezier(current_point.unwrap());
                for approximation_command in &approximation {
                    match *approximation_command {
                        Line { to } => {
                            try!(page.line_to(to));
                        }
                        Curve { control_1, control_2, to } => {
                            try!(page.curve_to(control_1, control_2, to));
                        }
                        _ => unreachable!()
                    }
                }
                current_point = Some(arc.to);
            }
        }
    }
    try!(page.fill());
    if let Some(error) = path.error() {
        println!("Error around path byte {}: {}.", error.position, error.reason);
    }
    Ok(())
}
