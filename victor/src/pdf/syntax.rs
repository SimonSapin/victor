//! File Structure
//! https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1877172

use itoa::write as itoa;
use std::borrow::Cow;
use std::io::{self, Write};
use super::object::Dictionary;

pub(crate) struct PdfFile {
    indirect_objects: Vec<Option<Vec<u8>>>,
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct IndirectObjectId(pub u32);

impl PdfFile {
    pub fn new() -> Self {
        PdfFile {
            indirect_objects: Vec::new(),
        }
    }

    pub fn add_stream(&mut self, meta: Dictionary, contents: Cow<[u8]>) -> IndirectObjectId {
        let meta = linked_dictionary! {
            &meta,
            "Length" => contents.len(),
        };

        let mut obj = Vec::new();
        meta.write(&mut obj).unwrap();
        obj.extend_from_slice(b"\nstream\n");
        obj.extend_from_slice(&contents);  // FIXME: avoid this copy?
        obj.extend_from_slice(b"\nendstream");
        self.add_indirect_object(obj)
    }

    pub fn add_dictionary(&mut self, dict: Dictionary) -> IndirectObjectId {
        let mut obj = Vec::new();
        dict.write(&mut obj).unwrap();
        self.add_indirect_object(obj)
    }

    pub fn set_dictionary(&mut self, id: IndirectObjectId, dict: Dictionary) {
        let mut obj = Vec::new();
        dict.write(&mut obj).unwrap();
        self.set_indirect_object(id, obj)
    }

    pub fn add_indirect_object(&mut self, serialized_contents: Vec<u8>) -> IndirectObjectId {
        self.indirect_objects.push(Some(serialized_contents));
        IndirectObjectId(self.indirect_objects.len() as u32)  // IDs start at 1
    }

    pub fn assign_indirect_object_id(&mut self) -> IndirectObjectId {
        self.indirect_objects.push(None);
        IndirectObjectId(self.indirect_objects.len() as u32)
    }

    pub fn set_indirect_object(&mut self, id: IndirectObjectId, serialized_contents: Vec<u8>) {
        self.indirect_objects[(id.0 - 1) as usize] = Some(serialized_contents)
    }
}

impl PdfFile {
    pub fn write<W: Write>(&self, catalog_id: IndirectObjectId, info_id: IndirectObjectId,
                           w: &mut W) -> io::Result<()> {
        let mut indirect_object_offsets;
        let startxref;
        {
            let mut w = CountingWrite {
                inner: w,
                bytes_written: 0,
            };
            w.write_all(b"%PDF-1.5\n%\xB5\xED\xAE\xFB\n")?;

            indirect_object_offsets = Vec::with_capacity(self.indirect_objects.len());
            for (object_id, contents) in (1..).zip(&self.indirect_objects) {
                // Indirect Objects
                // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1638996
                indirect_object_offsets.push(w.bytes_written as u32);
                itoa(&mut w, object_id)?;
                w.write_all(b" 0 obj\n")?;  // Generation number is always zero for us
                w.write_all(contents.as_ref().expect("Assigned indirect object was not set"))?;
                w.write_all(b"\nendobj\n")?;
            }

            startxref = w.bytes_written;
        }

        w.write_all(b"xref\n0 ")?;
        itoa(&mut *w, self.indirect_objects.len())?;
        w.write_all(b"\n0000000000 65535 f \n")?;
        let mut buffer: [u8; 20] = *b"0000000000 00000 n \n";
        for &offset in &indirect_object_offsets {
            itoa_zero_padded(offset, slice_to_10(&mut buffer));
            w.write_all(&buffer)?;
        }

        // PDF file trailer:
        // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1941947
        w.write_all(b"trailer\n")?;
        let trailer = dictionary! {
            "Size" => indirect_object_offsets.len(),
            "Root" => catalog_id,
            "Info" => info_id,
        };
        trailer.write(w)?;
        w.write_all(b"\nstartxref\n")?;
        itoa(&mut *w, startxref)?;
        w.write_all(b"\n%%EOF")?;
        Ok(())
    }
}

#[inline]
fn slice_to_10(buffer: &mut [u8; 20]) -> &mut [u8; 10] {
    let ptr = buffer as *mut [u8; 20] as *mut [u8; 10];
    unsafe {
        &mut *ptr
    }
}

fn itoa_zero_padded(mut value: u32, buffer: &mut [u8; 10]) {
    for byte in buffer.iter_mut().rev() {
        *byte = b"0123456789"[(value % 10) as usize];
        value /= 10;
    }
}

pub struct CountingWrite<'a, W: Write + 'a> {
    inner: &'a mut W,
    bytes_written: usize,
}

impl<'a, W: Write> Write for CountingWrite<'a, W> {
    #[inline]
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let result = self.inner.write(buffer);
        if let Ok(bytes) = result {
            self.bytes_written += bytes;
        }
        result
    }

    #[inline]
    fn write_all(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.bytes_written += buffer.len();
        // If this returns `Err` we can’t know how many bytes were actually written (if any)
        // but that doesn’t matter since we’re gonna abort the entire PDF generation anyway.
        self.inner.write_all(buffer)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
