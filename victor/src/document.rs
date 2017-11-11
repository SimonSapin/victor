use euclid;
use fonts::{Font, FontError};
use pdf::{InProgressDoc, InProgressPage};
use std::fs;
use std::io::{self, Write};
use std::path;
use std::sync::Arc;

/// Origin at top-left corner, unit `1px`
pub struct CssPx;

pub use euclid::rect;
pub use euclid::point2 as point;
pub type Length<U> = euclid::Length<f32, U>;
pub type Point<U> = euclid::TypedPoint2D<f32, U>;
pub type Size<U> = euclid::TypedSize2D<f32, U>;
pub type Rect<U> = euclid::TypedRect<f32, U>;
pub type GlyphId = u16;

#[derive(Copy, Clone, PartialEq)]
pub struct RGBA(pub f32, pub f32, pub f32, pub f32);

pub struct Document {
    in_progress: InProgressDoc,
}

pub struct Page<'doc> {
    in_progress: InProgressPage<'doc>,
}

pub struct TextRun {
    pub font: Arc<Font>,
    pub font_size: Length<CssPx>,
    pub origin: Point<CssPx>,
    pub glyph_ids: Vec<GlyphId>,
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
    pub fn set_color(&mut self, rgba: &RGBA) {
        self.in_progress.set_color(rgba)
    }

    pub fn paint_rectangle(&mut self, rect: &Rect<CssPx>) {
        self.in_progress.paint_rectangle(rect)
    }

    pub fn show_text(&mut self, text: &TextRun) -> Result<(), FontError> {
        self.in_progress.show_text(text)
    }
}
