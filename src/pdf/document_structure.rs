use std::io::{self, Write};
use pdf::file_structure::{PdfFile, CountingWriter, ObjectId};

fn px_to_pt(value: f32) -> f32 {
    // 96px = 1in = 72pt
    // value * 1px = value * 96px / 96 = value * 72pt / 96 = (value * 0.75) * 1pt
    value * 0.75
}

pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub fn render<W: Write>(items: &[(Rect, Color)], bounds: Rect, output: W) -> io::Result<W> {
    let mut pdf = try!(PdfDocument::new(output));
    // We map CSS pt to Poscript points (which is the default length unit in PDF).
    try!(pdf.render_page(px_to_pt(bounds.width), px_to_pt(bounds.height), |output| {
        for &(ref rect, ref color) in items {
            try!(write!(output, "{} {} {} sc {} {} {} {} re f\n",
                        color.r, color.g, color.b,
                        rect.x, rect.y, rect.width, rect.height))
        }
        Ok(())
    }));
    pdf.finish()
}

struct PdfDocument<W: Write> {
    file: PdfFile<W>,
    page_tree_id: ObjectId,
    page_objects_ids: Vec<ObjectId>,
}

impl<W: Write> PdfDocument<W> {
    fn new(output: W) -> io::Result<Self> {
        let mut file = try!(PdfFile::new(output));
        Ok(PdfDocument {
            page_tree_id: file.assign_object_id(),
            page_objects_ids: Vec::new(),
            file: file,
        })
    }

    fn render_page<F>(&mut self, width: f32, height: f32, render_contents: F) -> io::Result<()>
    where F: FnOnce(&mut CountingWriter<W>) -> io::Result<()> {
        let page_tree_id = self.page_tree_id;
        let page_id = self.file.assign_object_id();
        let contents_id = self.file.assign_object_id();
        self.page_objects_ids.push(page_id);
        try!(self.file.write_object(page_id, |output| {
            write!(
                output,
                "\
                << /Type /Page\n\
                   /Parent {page_tree}\n\
                   /Contents {contents}\n\
                   /MediaBox [ 0 0 {width} {height} ]\n\
                   /Resources << >>\n\
                >>\n\
                ",
                page_tree = page_tree_id,
                contents = contents_id,
                width = width,
                height = height
            )
        }));
        self.write_stream(contents_id, |output| {
            try!(write!(output, "/DeviceRGB cs /DeviceRGB CS 0.75 0 0 -0.75 0 {} cm\n", height));
            render_contents(output)
        })
    }

    /// Write a stream object.
    ///
    /// [§7.3.8 Stream Objects](https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1840319)
    fn write_stream<F>(&mut self, id: ObjectId, write_content: F) -> io::Result<()>
    where F: FnOnce(&mut CountingWriter<W>) -> io::Result<()> {
        let length_id = self.file.assign_object_id();
        let mut length = None;
        try!(self.file.write_object(id, |output| {
            try!(write!(output, "<< /Length {} >>\nstream\n", length_id));
            let start = output.position();
            try!(write_content(output));
            length = Some(output.position() - start);
            write!(output, "endstream\n")
        }));
        self.file.write_object(length_id, |output| write!(output, "{}\n", length.unwrap()))
    }

    fn finish(mut self) -> io::Result<W> {
        let page_objects_ids = &self.page_objects_ids;
        try!(self.file.write_object(self.page_tree_id, |output| {
            try!(write!(output, "<<  /Type /Pages\n"));
            try!(write!(output, "    /Count {}\n", page_objects_ids.len()));
            try!(write!(output, "    /Kids [ "));
            for &page_object_id in page_objects_ids {
                try!(write!(output, "{} ", page_object_id));
            }
            try!(write!(output, "]\n"));
            try!(write!(output, ">>\n"));
            Ok(())
        }));
        let page_tree_id = self.page_tree_id;
        let catalog_id = self.file.assign_object_id();
        try!(self.file.write_object(catalog_id, |output| {
            try!(write!(output, "<<  /Type /Catalog\n"));
            try!(write!(output, "    /Pages {}\n", page_tree_id));
            try!(write!(output, ">>\n"));
            Ok(())
        }));
        self.file.finish(catalog_id, None)
    }
}
