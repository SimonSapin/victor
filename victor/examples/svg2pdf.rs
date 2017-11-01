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
    let parser = xml::Parser::new();
    let doc = parser.parse_file(filename)?;
    let mut pdf = pdf::PdfDocument::create_file("out.pdf")?;
    pdf.write_page(900., 900., |page| {
        render_node(doc, page, &Style::default())
    })?;
    pdf.finish()?;
    Ok(())
}

#[derive(Default, Clone)]
struct Style {
    stroking: bool,
    filling: bool,
}

fn render_node<W: Write>(node: xml::Ref, page: &mut pdf::Page<W>, parent_style: &Style)
                         -> io::Result<()> {
    let mut style = parent_style.clone();
    page.save_state()?;
    if let Some(element) = node.as_element() {
        if let Some(attr) = element.attribute(&atom!("fill")) {
            if let Ok(c) = parse_color(attr) {
                style.filling = true;
                page.non_stroking_color(c.red, c.green, c.blue)?
            }
        }
        if let Some(attr) = element.attribute(&atom!("stroke")) {
            if let Ok(c) = parse_color(attr) {
                style.stroking = true;
                page.stroking_color(c.red, c.green, c.blue)?
            }
        }
        if let Some(attr) = element.attribute(&atom!("stroke-width")) {
            if let Ok(width) = CssParser::new(attr).parse_entirely(|p| p.expect_number()) {
                page.line_width(width)?
            }
        }
        if let Some(attr) = element.attribute(&atom!("transform")) {
            if let Ok((a, b, c, d, e, f)) = parse_transform(attr) {
                page.transform_matrix(a, b, c, d, e, f)?
            }
        }
        if element.data.name == qualname!(svg, "path") {
            if let Some(d_attribute) = element.attribute(&atom!("d")) {
                render_path(d_attribute, page, &style)?;
            }
        }
    }

    let mut link = node.first_child();
    while let Some(child) = link {
        render_node(child, page, &style)?;
        link = child.next_sibling()
    }
    page.restore_state()?;
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
        parser.expect_function_matching("matrix")?;
        parser.parse_nested_block(|parser| {
            let a = parser.expect_number()?;
            parser.expect_comma()?;
            let b = parser.expect_number()?;
            parser.expect_comma()?;
            let c = parser.expect_number()?;
            parser.expect_comma()?;
            let d = parser.expect_number()?;
            parser.expect_comma()?;
            let e = parser.expect_number()?;
            parser.expect_comma()?;
            let f = parser.expect_number()?;
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
                page.move_to(to)?;
                current_point = Some(to)
            }
            Line { to } => {
                page.line_to(to)?;
                current_point = Some(to)
            }
            Curve { control_1, control_2, to } => {
                page.curve_to(control_1, control_2, to)?;
                current_point = Some(to)
            }
            ClosePath => {
                page.close_path()?
            }
            EllipticalArc(arc) => {
                let approximation = arc.to_cubic_bezier(current_point.unwrap());
                for approximation_command in &approximation {
                    match *approximation_command {
                        Line { to } => {
                            page.line_to(to)?;
                        }
                        Curve { control_1, control_2, to } => {
                            page.curve_to(control_1, control_2, to)?;
                        }
                        _ => unreachable!()
                    }
                }
                current_point = Some(arc.to);
            }
        }
    }
    match (style.filling, style.stroking) {
        (true, true) => page.fill_and_stroke()?,
        (true, false) => page.fill()?,
        (false, true) => page.stroke()?,
        (false, false) => unreachable!(),
    }
    if let Some(error) = path.error() {
        println!("Error around path byte {}: {}.", error.position, error.reason);
    }
    Ok(())
}
