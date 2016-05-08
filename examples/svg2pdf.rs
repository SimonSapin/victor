#[macro_use] extern crate string_cache;
extern crate victor;

fn main() {
    render().unwrap()
}

fn render() -> victor::xml::Result<()> {
    let filename = match std::env::args().nth(1) {
        Some(arg) => std::path::PathBuf::from(arg),
        None => std::path::Path::new(file!()).parent().unwrap().join("rust-logo").join("rust-logo-blk.svg")
    };
    let parser = victor::xml::Parser::new();
    let doc = try!(parser.parse_file(filename));
    let mut pdf = try!(victor::pdf::document_structure::PdfDocument::create_file("out.pdf"));
    try!(pdf.write_page(144., 144., |page| {
        let selector = victor::SelectorList::parse("path[d]").unwrap();
        doc.iter(&mut |node| {
            if let Some(element) = node.as_element() {
                if selector.matches(element) {
                    let attribute = element.attribute(&atom!("d")).unwrap();
                    let mut path = victor::svg::path::parse(attribute).simplify();
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
                }
            }
            Ok(())
        })
    }));
    try!(pdf.finish());
    Ok(())
}
