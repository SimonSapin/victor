use fonts::FontError;
use primitives::{CssPx, Size, Rect, RGBA, TextRun};
use self::convert::{InProgressDoc, InProgressPage};
use std::fs;
use std::io::{self, Write};
use std::path;

mod convert;

pub struct Document {
    in_progress: InProgressDoc,
}

pub struct Page<'doc> {
    in_progress: InProgressPage<'doc>,
}

impl Document {
    pub fn new() -> Self {
        Document {
            in_progress: InProgressDoc::new(),
        }
    }

    pub fn add_page(&mut self, size: Size<CssPx>) -> Page {
        Page {
            in_progress: InProgressPage::new(&mut self.in_progress, size),
        }
    }

    /// Encode this document to PDF and write it into the file with the given name.
    pub fn write_to_pdf_file<P: AsRef<path::Path>>(self, filename: P) -> Result<(), io::Error> {
        self.write_to_pdf(&mut io::BufWriter::new(fs::File::create(filename)?))
    }

    /// Encode this document to PDF and return a vector of bytes
    pub fn write_to_pdf_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // Unwrap io::Result because <io::Write for Vec<u8>> never emits errors.
        self.write_to_pdf(&mut bytes).unwrap();
        bytes
    }

    /// Encode this document to PDF and write it to the given stream.
    ///
    /// Note: this may do many write calls.
    /// If a stream is backed by costly system calls (such as `File` or `TcpStream`),
    /// this method will likely perform better with that stream wrapped in `BufWriter`.
    ///
    /// See also the `write_to_pdf_file` and `write_to_pdf_bytes` methods.
    pub fn write_to_pdf<W: Write>(self, stream: &mut W) -> Result<(), io::Error> {
        Ok(self.in_progress.finish().save_to(stream)?)
    }
}

impl<'doc> Page<'doc> {
    pub fn set_color(&mut self, rgba: &RGBA) -> &mut Self {
        self.in_progress.set_color(rgba);
        self
    }

    pub fn paint_rectangle(&mut self, rect: &Rect<CssPx>) -> &mut Self {
        self.in_progress.paint_rectangle(rect);
        self
    }

    pub fn show_text(&mut self, text: &TextRun) -> Result<&mut Self, FontError> {
        self.in_progress.show_text(text)?;
        Ok(self)
    }
}
