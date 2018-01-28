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

    println!("{}", postscript_name(bytes, table_directory)?);

    Ok(())
}

fn postscript_name(bytes: &[u8], table_directory: Slice<TableDirectoryEntry>)
                   -> Result<String, FontError> {
    /// Macintosh encodings seem to be ASCII-compatible, and a PostScript name is within ASCII
    fn decode_macintosh(string_bytes: &[u8]) -> String {
        String::from_utf8_lossy(string_bytes).into_owned()
    }

    /// Latin-1 range only
    fn decode_ucs2(string_bytes: &[u8]) -> String {
        string_bytes.chunks(2).map(|chunk| {
            if chunk.len() < 2 || chunk[0] != 0 {
                '\u{FFFD}'
            } else {
                chunk[1] as char
            }
        }).collect::<String>()
    };

    let naming_table_header = table_directory.find_table::<NamingTableHeader>(bytes)?;
    let name_records = Slice::new(
        naming_table_header.followed_by::<NameRecord>(),
        naming_table_header.count().read_from(bytes)?,
    );
    let string_storage_start: Position<()> = naming_table_header.offset(
        naming_table_header.string_offset().read_from(bytes)?
    );
    let string_bytes = |record: Position<NameRecord>| {
        Slice::<u8>::new(
            string_storage_start.offset(record.string_offset().read_from(bytes)?),
            record.length().read_from(bytes)?
        ).read_from(bytes)
    };

    for record in name_records {
        const POSTSCRIPT_NAME: u16 = 6;
        if record.name_id().read_from(bytes)? != POSTSCRIPT_NAME {
            continue
        }

        const MACINTOSH: u16 = 1;
        const MICROSOFT: u16 = 3;
        const UNICODE_BMP: u16 = 1;
        let postscript_name = match (
            record.platform_id().read_from(bytes)?,
            record.encoding_id().read_from(bytes)?,
        ) {
            (MACINTOSH, _) => decode_macintosh(string_bytes(record)?),
            (MICROSOFT, UNICODE_BMP) => decode_ucs2(string_bytes(record)?),
            _ => continue,
        };
        return Ok(postscript_name)
    }

    Err(FontError::NoSupportedPostscriptName)
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
