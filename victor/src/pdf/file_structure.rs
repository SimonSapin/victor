/*!

# PDF file structure

This module takes care of the lowest level of PDF syntax, as specified in [§7.5 File Structure].
A PDF file is made of, in order:

* A header, which for our purpose is a fixed byte string.
* A body, made of a sequence of indirect objects ([§7.3.10 Indirect Objects]).
  Each indirect object has:
  * An object identifier: an object number (a positive integer) and a generation number.
    For our purpose (without incremental updates), the generation number is always zero.
  * Some content: as far as this module is concerned, arbitrary bytes.
    The syntax of various types of objects is specified in [§7.3 Objects].
* A cross-reference table that indicates the position (in bytes since the start of the file)
  of each indirect object.
  This table can contain multiple sections, but we only ever write one.
* A trailer, the entry point of the PDF file.
  It is a dictionary object
  that contains (most importantly for us) reference to some indirect objects.


## Abstraction

This module abstracts most of the above and provides an API where the user:

* creates a new PDF files (this writes the header),
* writes a number of indirect objects to it (this keeps track of their positions and identifiers),
* then ends by providing the identifier of the [§7.7.2 Document Catalog] dictionary object
  and optionally that of the [§14.3.3 Document Information Dictionary] object
  (this writes the cross-reference table and the trailer).

Methods whose return types is `io::Result<_>` will return an `Err(io_error)` value
when writing to the underlying byte stream does.
When that happens, it is unspecified how much has been successfully written before the error
so the `PdfFile` should not be used anymore.

[§7.5 File Structure]: https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1877172
[§7.3.10 Indirect Objects]: https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1638996
[§7.3 Objects]: https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1965566
[§7.3.7 Dictionary Objects]: https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1638921
[§7.7.2 Document Catalog]: https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#G6.1926881
[§14.3.3 Document Information Dictionary]: https://wwwimages2.adobe.com/content/dam/Adobe/en/devnet/pdf/pdfs/PDF32000_2008.pdf#M13.9.24061.1Heading.82.Info.Dictionaries

*/

use std::fmt;
use std::io::{self, Write};

/// An abstraction for the low-level PDF file structure.
///
/// See this module’s doc-comment.
pub struct PdfFile<W: Write> {
    output: CountingWriter<W>,

    /// Indices in this vector are object numbers.
    /// Object zero is reserved in PDF syntax and always `None` here.
    /// For other indices,
    /// `None` indicates an object not written yet whose ID is already reserved.
    /// `Some(n)` indicates the number of bytes from the start of the file to the start of this object.
    ///
    /// When `.finish()` is called, only object zero should still be `None`.
    objects_positions: Vec<Option<u64>>,
}

/// The identifier of an indirect object.
///
/// Since the generation number is always zero for us, this only contains the object number.
#[derive(Copy, Clone)]
pub struct ObjectId(usize);

impl fmt::Display for ObjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{} 0 R", self.0)
    }
}

impl<W: Write> PdfFile<W> {
    /// Write a PDF header to a `std::io::Write` byte stream and (unless there’s an IO error)
    /// return it wrapped in a `PdfFile` value.
    pub fn new(mut output: W) -> io::Result<Self> {
        // FIXME: Find out the lowest version that contains the features we’re using.
        output.write_all(b"%PDF-1.7\n%\xB5\xED\xAE\xFB\n")?;
        Ok(PdfFile {
            output: CountingWriter {
                inner: output,
                bytes_written: 0,
            },
            objects_positions: vec![None],
        })
    }

    /// Assign a object identifier, to be used later with the `write_object` method.
    ///
    /// This allows writing an indirect reference to an object that is not written yet.
    pub fn assign_object_id(&mut self) -> ObjectId {
        let object_number = self.objects_positions.len();
        self.objects_positions.push(None);
        ObjectId(object_number)
    }

    /// Write an object with an identifier previously assigned by the `assign_object_id` method.
    pub fn write_object<F>(&mut self, id: ObjectId, write_content: F) -> io::Result<()>
    where F: FnOnce(&mut CountingWriter<W>) -> io::Result<()> {
        assert!(self.objects_positions[id.0].is_none(), "object {} is already written", id.0);
        self.objects_positions[id.0] = Some(self.output.position());

        write!(self.output, "{} 0 obj\n", id.0)?;
        write_content(&mut self.output)?;
        write!(self.output, "endobj\n")?;
        Ok(())
    }

    /// Write the cross-reference table and trailer, then return the underlying byte stream.
    pub fn finish(mut self, document_catalog: ObjectId,
              document_information_dictionary: Option<ObjectId>)
              -> io::Result<W> {

        let startxref = self.output.position();
        write!(self.output, "xref\n\
                             0 {}\n", self.objects_positions.len())?;
        // Object 0 is the head of the linked list of free objects, but we don’t use free objects.
        write!(self.output, "0000000000 65535 f \n")?;
        // Use [1..] to skip object 0 in self.objects_positions.
        for position in &self.objects_positions[1..] {
            let bytes = position.expect("an object was assigned but not written");
            write!(self.output, "{:010} 00000 n \n", bytes)?;
        }

        write!(self.output, "trailer\n\
                             << /Size {}\n\
                                /Root {}\n", self.objects_positions.len(), document_catalog)?;
        if let Some(info) = document_information_dictionary {
            write!(self.output, "/Info {}\n", info)?;
        }
        write!(self.output, ">>\n\
                             startxref\n\
                             {}\n\
                             %%EOF\n", startxref)?;

        Ok(self.output.inner)
    }
}

pub struct CountingWriter<W: Write> {
    inner: W,
    bytes_written: u64,
}

impl<W: Write> CountingWriter<W> {
    /// Return the current position, measured in bytes from the start of the file.
    pub fn position(&self) -> u64 {
        self.bytes_written
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let result = self.inner.write(buffer);
        if let Ok(bytes) = result {
            self.bytes_written += bytes as u64;
        }
        result
    }

    fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.bytes_written += buffer.len() as u64;
        // If this returns `Err` we can’t know how many bytes were actually written (if any)
        // but that doesn’t matter since we’re gonna abort the entire PDF generation anyway.
        self.inner.write_all(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
