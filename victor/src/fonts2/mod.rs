mod parsing;
mod tables;

use std::fmt::{self, Write};
use fonts2::tables::*;
use fonts2::parsing::*;

pub use fonts::FontError;

pub fn read(bytes: &[u8]) -> Result<(), FontError> {
    let offset_table = Position::<OffsetSubtable>::initial();
    let scaler_type = offset_table.scaler_type().read_from(bytes)?;
    const TRUETYPE: u32 = 0x74727565;  // "true" in big-endian
    if scaler_type != TRUETYPE && scaler_type != 0x_0001_0000 {
        Err(FontError::UnsupportedFormat)?
    }

    let table_directory = Slice::new(
        offset_table.followed_by::<TableDirectoryEntry>(),
        offset_table.table_count().read_from(bytes)?,
    );
    for table in table_directory {
        let tag = table.tag().read_from(bytes)?;
        println!("{:?}", tag)
    }

    let maxp = table_directory.find_table::<MaximumProfile>(bytes)?;
    println!("{}", maxp.num_glyphs().read_from(bytes)?);

    Ok(())
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Tag(pub [u8; 4]);

trait SfntTable {
    const TAG: Tag;
}

impl Slice<TableDirectoryEntry> {
    fn find_table<T: SfntTable>(&self, bytes: &[u8]) -> Result<Position<T>, FontError> {
        let search = self.binary_search_by_key(&T::TAG, |entry| entry.tag().read_from(bytes))?;
        let entry = search.ok_or(FontError::MissingTable)?;
        let offset = entry.table_offset().read_from(bytes)?;
        Ok(Position::<OffsetSubtable>::initial().offset(offset))
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for &b in &self.0 {
            // ASCII printable or space
            f.write_char(if b' ' <= b && b <= b'~' { b } else { b'?' } as char)?
        }
        Ok(())
    }
}

impl ReadFromBytes for Tag {
    fn read_from(bytes: &[u8]) -> Result<Self, FontError> {
        ReadFromBytes::read_from(bytes).map(Tag)
    }
}
