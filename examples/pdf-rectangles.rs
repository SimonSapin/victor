extern crate victor;

fn main() {
    render().unwrap()
}

fn render() -> std::io::Result<()> {
    let mut pdf = victor::pdf::document_structure::PdfDocument::create_file("out.pdf")?;
    pdf.write_page(800., 600., |page| {
        page.non_stroking_color(0., 1., 0.)?;
        page.rectangle(100., 100., 200., 200.)?;
        page.fill()?;

        page.non_stroking_color(0., 0., 1.)?;
        page.rectangle(200., 150., 150., 250.)?;
        page.fill()?;
        Ok(())
    })?;
    pdf.finish()?;
    Ok(())
}
