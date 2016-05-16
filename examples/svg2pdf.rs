extern crate cssparser;
#[macro_use] extern crate string_cache;
extern crate victor;

use cssparser::{Color, RGBA};
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
        None => Path::new(file!()).parent().unwrap().join("svg").join("rust-logo-blk.svg")
    };
    let doc = try!(xml::Node::parse_file(filename));
    let mut pdf = try!(pdf::PdfDocument::create_file("out.pdf"));
    let cursor = doc.cursor();
    try!(pdf.write_page(900., 900., |page| {
        render_element(&cursor, page, &Style::default())
    }));
    try!(pdf.finish());
    Ok(())
}

#[derive(Default, Clone)]
struct Style {
    stroking: bool,
    filling: bool,
}

fn render_element<W: Write>(cursor: &xml::Cursor, page: &mut pdf::Page<W>, parent_style: &Style)
                            -> io::Result<()> {
    let element = cursor.element();
    let mut style = parent_style.clone();
    try!(page.save_state());

    if let Some(attr) = element.attribute(&atom!("fill")) {
        if let Ok(c) = parse_color(attr) {
            style.filling = true;
            try!(page.non_stroking_color(c.red, c.green, c.blue))
        }
    }
    if let Some(attr) = element.attribute(&atom!("stroke")) {
        if let Ok(c) = parse_color(attr) {
            style.stroking = true;
            try!(page.stroking_color(c.red, c.green, c.blue))
        }
    }
    if let Some(attr) = element.attribute(&atom!("stroke-width")) {
        if let Ok(width) = CssParser::new(attr).parse_entirely(|p| p.expect_number()) {
            try!(page.line_width(width))
        }
    }
    if let Some(attr) = element.attribute(&atom!("transform")) {
        if let Ok((a, b, c, d, e, f)) = parse_transform(attr) {
            try!(page.transform_matrix(a, b, c, d, e, f))
        }
    }
    if element.name == qualname!(svg, "path") {
        if let Some(d_attribute) = element.attribute(&atom!("d")) {
            try!(render_path(d_attribute, page, &style));
        }
    }

    let mut child_cursor = cursor.clone();
    if child_cursor.first_child_element() {
        try!(render_element(&child_cursor, page, &style));
        while child_cursor.next_sibling_element() {
            try!(render_element(&child_cursor, page, &style));
        }
    }

    try!(page.restore_state());
    Ok(())
}

fn parse_color(s: &str) -> Result<RGBA, ()> {
    match CssParser::new(s).parse_entirely(Color::parse) {
        Ok(Color::RGBA(c)) => Ok(c),
        Ok(Color::CurrentColor) | Err(()) => Err(())
    }
}

fn parse_transform(s: &str) -> Result<(f32, f32, f32, f32, f32, f32), ()> {
    CssParser::new(s).parse_entirely(|parser| {
        try!(parser.expect_function_matching("matrix"));
        parser.parse_nested_block(|parser| {
            let a = try!(parser.expect_number());
            try!(parser.expect_comma());
            let b = try!(parser.expect_number());
            try!(parser.expect_comma());
            let c = try!(parser.expect_number());
            try!(parser.expect_comma());
            let d = try!(parser.expect_number());
            try!(parser.expect_comma());
            let e = try!(parser.expect_number());
            try!(parser.expect_comma());
            let f = try!(parser.expect_number());
            Ok((a, b, c, d, e, f))
        })
    })
}

fn render_path<W: Write>(d_attribute: &str, page: &mut pdf::Page<W>, style: &Style)
                         -> io::Result<()> {
    if !(style.filling || style.stroking) {
        return Ok(())
    }

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
    match (style.filling, style.stroking) {
        (true, true) => try!(page.fill_and_stroke()),
        (true, false) => try!(page.fill()),
        (false, true) => try!(page.stroke()),
        (false, false) => unreachable!(),
    }
    if let Some(error) = path.error() {
        println!("Error around path byte {}: {}.", error.position, error.reason);
    }
    Ok(())
}
