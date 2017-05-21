use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use svg::geometry::Pair;
use pdf::file_structure::{PdfFile, CountingWriter, ObjectId};

fn px_to_pt(value: f64) -> f64 {
    // 96px = 1in = 72pt
    // value * 1px = value * 96px / 96 = value * 72pt / 96 = (value * 0.75) * 1pt
    value * 0.75
}

pub struct PdfDocument<W: Write> {
    file: PdfFile<W>,
    page_tree_id: ObjectId,
    page_objects_ids: Vec<ObjectId>,
    font_objects_ids: Vec<ObjectId>,
}

impl PdfDocument<BufWriter<File>> {
    pub fn create_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        PdfDocument::new(BufWriter::new(File::create(path)?))
    }
}

impl<W: Write> PdfDocument<W> {
    pub fn new(output: W) -> io::Result<Self> {
        let mut file = PdfFile::new(output)?;
        Ok(PdfDocument {
            page_tree_id: file.assign_object_id(),
            file: file,
            page_objects_ids: Vec::new(),
            font_objects_ids: Vec::new(),
        })
    }

    pub fn write_page<F>(&mut self, width: f64, height: f64, render_contents: F) -> io::Result<()>
    where F: FnOnce(&mut Page<W>) -> io::Result<()> {
        // We map CSS pt to Poscript points (which is the default length unit in PDF).
        let width = px_to_pt(width);
        let height = px_to_pt(height);

        let page_tree_id = self.page_tree_id;
        let page_id = self.file.assign_object_id();
        let contents_id = self.file.assign_object_id();
        self.page_objects_ids.push(page_id);
        self.file.write_object(page_id, |output| {
            write!(
                output,
                "\
                << /Type /Page\n\
                   /Parent {page_tree}\n\
                   /Contents {contents}\n\
                   /MediaBox [ 0 0 {width} {height} ]\n\
                >>\n\
                ",
                page_tree = page_tree_id,
                contents = contents_id,
                width = width,
                height = height
            )
        })?;
        self.write_stream(contents_id, |output| {
            // 0.75 (like in px_to_pt) makes the coordinate system be in CSS px units.
            write!(output, "/DeviceRGB cs /DeviceRGB CS 0.75 0 0 -0.75 0 {} cm\n", height)?;
            render_contents(&mut Page {
                output: output,
            })
        })
    }

    /// Write a stream object.
    ///
    /// [§7.3.8 Stream Objects](https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1840319)
    fn write_stream<F>(&mut self, id: ObjectId, write_content: F) -> io::Result<()>
    where F: FnOnce(&mut CountingWriter<W>) -> io::Result<()> {
        let length_id = self.file.assign_object_id();
        let mut length = None;
        self.file.write_object(id, |output| {
            write!(output, "<< /Length {} >>\nstream\n", length_id)?;
            let start = output.position();
            write_content(output)?;
            length = Some(output.position() - start);
            write!(output, "endstream\n")
        })?;
        self.file.write_object(length_id, |output| write!(output, "{}\n", length.unwrap()))
    }

    // FIXME: higher-level API
    pub fn write_font<F>(&mut self, write_content: F) -> io::Result<FontId>
    where F: FnOnce(&mut CountingWriter<W>) -> io::Result<()> {
        let font_id = FontId(self.font_objects_ids.len());
        let font_object_id = self.file.assign_object_id();
        self.font_objects_ids.push(font_object_id);
        self.file.write_object(font_object_id, write_content)?;
        Ok(font_id)
    }

    pub fn finish(mut self) -> io::Result<W> {
        let page_objects_ids = &self.page_objects_ids;
        let font_objects_ids = &self.font_objects_ids;
        self.file.write_object(self.page_tree_id, |output| {
            write!(output, "<< /Type /Pages\n\
                               /Count {}\n\
                               /Kids [ ", page_objects_ids.len())?;
            for &id in page_objects_ids {
                write!(output, "{} ", id)?;
            }
            write!(output, "]\n\
                            /Resources << /Font << ")?;
            for (i, &id) in font_objects_ids.iter().enumerate() {
                write!(output, "/F{} {}", i, id)?;
            }
            write!(output, ">> >>\n>>\n")?;
            Ok(())
        })?;
        let page_tree_id = self.page_tree_id;
        let catalog_id = self.file.assign_object_id();
        self.file.write_object(catalog_id, |output| {
            write!(output, "<<  /Type /Catalog\n\
                                /Pages {}\n\
                                >>\n", page_tree_id)?;
            Ok(())
        })?;
        let info_id = self.file.assign_object_id();
        self.file.write_object(info_id, |output| {
            write!(output, "<< /Producer (Victor (https://github.com/SimonSapin/victor)) >>\n")
        })?;
        self.file.finish(catalog_id, Some(info_id))
    }

    pub fn low_level_objects(&mut self) -> &mut PdfFile<W> {
        &mut self.file
    }
}

#[derive(Copy, Clone)]
pub struct FontId(usize);

impl fmt::Display for FontId {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "/F{}", self.0)
    }
}

pub struct Page<'a, W: 'a + Write> {
    output: &'a mut CountingWriter<W>,
}

impl<'a, W: Write> Page<'a, W> {
    pub fn save_state(&mut self) -> io::Result<()> {
        write!(self.output, "q\n")
    }

    pub fn restore_state(&mut self) -> io::Result<()> {
        write!(self.output, "Q\n")
    }

    pub fn transform_matrix(&mut self, a: f32, b: f32, c: f32, d: f32, e: f32, f: f32)
                            -> io::Result<()> {
        write!(self.output, "{} {} {} {} {} {} cm\n", a, b, c, d, e, f)
    }

    pub fn line_width(&mut self, width: f32) -> io::Result<()> {
        write!(self.output, "{} w\n", width)
    }

    pub fn non_stroking_color(&mut self, red: f32, green: f32, blue: f32) -> io::Result<()> {
        write!(self.output, "{} {} {} sc\n", red, green, blue)
    }

    pub fn stroking_color(&mut self, red: f32, green: f32, blue: f32) -> io::Result<()> {
        write!(self.output, "{} {} {} SC\n", red, green, blue)
    }

    pub fn move_to(&mut self, point: Pair) -> io::Result<()> {
        write!(self.output, "{} {} m\n", point.x, point.y)
    }

    pub fn line_to(&mut self, point: Pair) -> io::Result<()> {
        write!(self.output, "{} {} l\n", point.x, point.y)
    }

    pub fn curve_to(&mut self, control_1: Pair, control_2: Pair, end: Pair) -> io::Result<()> {
        write!(self.output, "{} {} {} {} {} {} c\n",
               control_1.x, control_1.y, control_2.x, control_2.y, end.x, end.y)
    }

    pub fn close_path(&mut self) -> io::Result<()> {
        write!(self.output, "h\n")
    }

    pub fn rectangle(&mut self, x: f64, y: f64, width: f64, height: f64) -> io::Result<()> {
        write!(self.output, "{} {} {} {} re\n", x, y, width, height)
    }

    pub fn fill(&mut self) -> io::Result<()> {
        write!(self.output, "f\n")
    }

    pub fn stroke(&mut self) -> io::Result<()> {
        write!(self.output, "S\n")
    }

    pub fn fill_and_stroke(&mut self) -> io::Result<()> {
        write!(self.output, "B\n")
    }

    pub fn low_level_page_stream(&mut self) -> &mut CountingWriter<W> {
        self.output
    }
}
