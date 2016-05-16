extern crate victor;

use std::io::Write;

fn main() {
    render().unwrap()
}

fn render() -> std::io::Result<()> {
    let mut pdf = try!(victor::pdf::document_structure::PdfDocument::create_file("out.pdf"));
    let encoding_id;
    {
        let objects = pdf.low_level_objects();
        encoding_id = objects.assign_object_id();
        try!(objects.write_object(encoding_id, |object| {
            write!(object, "<< /Type /Encoding /Differences [ 254 /eacute /twosuperior ] >>\n")
        }));
    }
    let mut font_ids = [None; 14];
    for (i, name) in CORE_14.iter().enumerate() {
        font_ids[i] = Some(try!(pdf.write_font(|object| {
            write!(object,
                   "<< /Type /Font /SubType /Type1 /BaseFont /{} /Encoding {} >>\n",
                   name, encoding_id)
        })));
    }
    try!(pdf.write_page(600., 400., |page| {
        let stream = page.low_level_page_stream();
        // FIXME: find a way to get upright text without negative size?
        // but keeping coordinate with Y going down.
        try!(write!(stream, "\
            BT\n\
                -100 Tz\n\
                -20 TL\n\
                10 20 Td\n"));
        for (i, name) in CORE_14.iter().enumerate() {
            try!(write!(stream, "\
                {} -12 Tf\n\
                [ ({} is one of the 14 \"standard\" fonts. Some non-ASCII: ) <FEFF> ] TJ T*\n\
            ", font_ids[i].unwrap(), name));
        }
        try!(write!(stream, "ET\n"));
        Ok(())
    }));
    try!(pdf.finish());
    Ok(())
}

static CORE_14: [&'static str; 14] = [
    "Courier",
    "Courier-Oblique",
    "Courier-Bold",
    "Courier-BoldOblique",
    "Helvetica",
    "Helvetica-Oblique",
    "Helvetica-Bold",
    "Helvetica-BoldOblique",
    "Times-Roman",
    "Times-Italic",
    "Times-Bold",
    "Times-BoldItalic",
    "Symbol",
    "ZapfDingbats",
];
