extern crate victor;

use std::io;
use victor::pdf::document_structure::{PdfDocument, Rect, Color};

fn main() {
    render().unwrap()
}

fn render() -> io::Result<()> {
    let mut pdf = try!(PdfDocument::create_file("out.pdf"));
    try!(pdf.write_page(800., 600., |page| {
        try!(page.paint_rectangle(Rect { x: 100., y: 100., width: 200., height: 200. },
                                  Color { r: 0., g: 1., b: 0.}));
        try!(page.paint_rectangle(Rect { x: 200., y: 150., width: 150., height: 250. },
                                  Color { r: 0., g: 0., b: 1.}));
        Ok(())
    }));
    try!(pdf.finish());
    Ok(())
}
