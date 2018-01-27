mod parsing;
mod tables;

use std::fmt::{self, Write};
use fonts::FontError;
use fonts2::tables::*;
use fonts2::parsing::*;


pub fn read(bytes: &[u8]) -> Result<(), FontError> {
    let offset_table = Position::<OffsetSubtable>::initial();
    let scaler_type = offset_table.scaler_type().read_from(bytes);
    const TRUETYPE: u32 = 0x74727565;  // "true" in big-endian
    if scaler_type != TRUETYPE && scaler_type != 0x_0001_0000 {
        Err(FontError::UnsupportedFormat)?
    }

    let table_directory = Slice {
        start: offset_table.followed_by::<TableDirectoryEntry>(),
        count: offset_table.table_count().read_from(bytes) as usize,
    };
    for table in table_directory {
        let tag = table.tag().read_from(bytes);
        println!("{:?}", tag)
    }

    Ok(())
}

struct Tag(pub [u8; 4]);

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
    fn read_from(bytes: &[u8]) -> Self {
        Tag(ReadFromBytes::read_from(bytes))
    }
}
