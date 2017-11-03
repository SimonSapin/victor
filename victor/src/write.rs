use errors::VictorError;
use display_lists;
use pdf;
use std::fs;
use std::io::{self, Write, Seek};
use std::path;

impl display_lists::Document {
    /// Encode this document to PDF and write it into the file with the given name.
    pub fn write_to_pdf_file<P: AsRef<path::Path>>(&self, filename: P) -> Result<(), VictorError> {
        self.write_to_pdf(&mut io::BufWriter::new(fs::File::create(filename)?))
    }

    /// Encode this document to PDF and return a vector of bytes
    pub fn write_to_pdf_bytes(&self) -> Result<Vec<u8>, VictorError> {
        struct Seek(Vec<u8>);

        impl io::Seek for Seek {
            #[inline]
            fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
                if pos == io::SeekFrom::Current(0) {
                    Ok(self.0.len() as u64)
                } else {
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "cannot seek in Vec<u8>"))
                }
            }
        }

        impl io::Write for Seek {
            #[inline] fn write(&mut self, buf: &[u8]) -> io::Result<usize> { self.0.write(buf) }
            #[inline] fn write_all(&mut self, buf: &[u8]) -> io::Result<()> { self.0.write_all(buf) }
            #[inline] fn flush(&mut self) -> io::Result<()> { self.0.flush() }
        }

        let mut bytes = Seek(Vec::new());
        self.write_to_pdf(&mut bytes)?;
        Ok(bytes.0)
    }

    /// Encode this document to PDF and write it to the given stream.
    ///
    /// Note: this may do many write calls.
    /// If a stream is backed by costly system calls (such as `File` or `TcpStream`),
    /// this method will likely perform better with that stream wrapped in `BufWriter`.
    ///
    /// See also the `write_to_png_file` method.
    pub fn write_to_pdf<W: Write + Seek>(&self, stream: &mut W) -> Result<(), VictorError> {
        Ok(pdf::from_display_lists(self)?.save_to(stream)?)
    }
}
