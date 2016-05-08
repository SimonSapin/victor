extern crate victor;

fn main() {
    render().unwrap()
}

fn render() -> std::io::Result<()> {
    let mut pdf = try!(victor::pdf::document_structure::PdfDocument::create_file("out.pdf"));
    try!(pdf.write_page(800., 600., |page| {
        try!(page.non_stroking_color(0., 1., 0.));
        try!(page.rectangle(100., 100., 200., 200.));
        try!(page.fill());

        try!(page.non_stroking_color(0., 0., 1.));
        try!(page.rectangle(200., 150., 150., 250.));
        try!(page.fill());
        Ok(())
    }));
    try!(pdf.finish());
    Ok(())
}
