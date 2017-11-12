//! File Structure
//! https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1877172

use itoa::write as itoa;
use std::borrow::Cow;
use std::io::{self, Write};
use super::object::Dictionary;

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct IndirectObjectId(pub u32);

// IDs start at 1. The first few indirect objects are always the same in Victor.
const FIRST_ID: IndirectObjectId = IndirectObjectId(1);
pub(crate) const PAGE_TREE_ID: IndirectObjectId = IndirectObjectId(1);
const CATALOG_ID: IndirectObjectId = IndirectObjectId(2);
const INFO_ID: IndirectObjectId = IndirectObjectId(3);
const FIRST_AVAILABLE_ID: IndirectObjectId = IndirectObjectId(4);

pub(crate) struct BasicObjects<'a> {
    pub page_tree: Dictionary<'a>,
    pub catalog: Dictionary<'a>,
    pub info: Dictionary<'a>,
}

pub(crate) struct PdfFile {
    indirect_objects: Vec<Vec<u8>>,
    next_id: IndirectObjectId,
}

impl PdfFile {
    pub fn new() -> Self {
        PdfFile {
            indirect_objects: Vec::new(),
            next_id: FIRST_AVAILABLE_ID,
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

    pub fn add_indirect_object(&mut self, serialized_contents: Vec<u8>) -> IndirectObjectId {
        self.indirect_objects.push(serialized_contents);
        let id = self.next_id;
        self.next_id.0 += 1;
        id
    }

    pub fn write<W: Write>(&self, w: &mut W, basic_objects: &BasicObjects) -> io::Result<()> {
        let total_indirect_object_count =
            (FIRST_AVAILABLE_ID.0 - FIRST_ID.0) as usize +
            self.indirect_objects.len();
        let mut indirect_object_offsets = Vec::with_capacity(total_indirect_object_count);
        let startxref;
        {
            let mut w = CountingWrite {
                inner: w,
                bytes_written: 0,
            };
            w.write_all(b"%PDF-1.5\n%\xB5\xED\xAE\xFB\n")?;

            // Indirect Objects
            // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1638996
            let mut next_object_id = FIRST_ID;
            for &(object_id, dictionary) in &[
                (PAGE_TREE_ID, &basic_objects.page_tree),
                (CATALOG_ID, &basic_objects.catalog),
                (INFO_ID, &basic_objects.info),
            ] {
                assert_eq!(next_object_id, object_id);
                next_object_id.0 += 1;

                indirect_object_offsets.push(w.bytes_written as u32);
                itoa(&mut w, object_id.0)?;
                w.write_all(b" 0 obj\n")?;  // Generation number is always zero for us
                dictionary.write(&mut w)?;
                w.write_all(b"\nendobj\n")?;
            }
            assert_eq!(next_object_id, FIRST_AVAILABLE_ID);
            for contents in &self.indirect_objects {
                let object_id = next_object_id;
                next_object_id.0 += 1;

                indirect_object_offsets.push(w.bytes_written as u32);
                itoa(&mut w, object_id.0)?;
                w.write_all(b" 0 obj\n")?;  // Generation number is always zero for us
                w.write_all(contents)?;
                w.write_all(b"\nendobj\n")?;
            }

            startxref = w.bytes_written;
        }
        assert_eq!(total_indirect_object_count, indirect_object_offsets.len());

        // Cross-reference table
        // https://www.adobe.com/content/dam/acom/en/devnet/pdf/PDF32000_2008.pdf#G6.1839814

        // Add 1 for the mandatory free object with ID zero.
        let xref_table_size = total_indirect_object_count + 1;
        w.write_all(b"xref\n0 ")?;
        itoa(&mut *w, xref_table_size)?;
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
            "Size" => xref_table_size,
            "Root" => CATALOG_ID,
            "Info" => INFO_ID,
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
