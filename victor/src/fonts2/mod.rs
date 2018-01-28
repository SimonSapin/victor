mod cmap;
mod parsing;
mod tables;
mod types;

use euclid;
use fonts::{FontError, GlyphId};
use fonts2::cmap::Cmap;
use fonts2::parsing::*;
use fonts2::tables::*;
use fonts2::types::{Tag, Em, FontDesignUnit};
use std::borrow::Cow;
use std::fmt;

pub struct Font {
    bytes: Cow<'static, [u8]>,
    postscript_name: String,
    cmap: Cmap,
    metrics: Metrics,
}

#[derive(Debug)]
struct Metrics {
    num_glyphs: u16,
    font_design_units_per_em: euclid::TypedScale<u16, Em, FontDesignUnit>,
}

#[cfg(target_pointer_width = "64")]
fn _assert_size_of() {
    let _ = ::std::mem::transmute::<Cmap, [u8; 36]>;
    let _ = ::std::mem::transmute::<Metrics, [u8; 4]>;
    let _ = ::std::mem::transmute::<Font, [u8; 96]>;
}

impl Font {
    pub fn parse<B: Into<Cow<'static, [u8]>>>(bytes: B) -> Result<Self, FontError> {
        Self::parse_cow(bytes.into())
    }

    fn parse_cow(bytes: Cow<'static, [u8]>) -> Result<Self, FontError> {
        let postscript_name;
        let metrics;
        let cmap;
        {
            let bytes: &[u8] = &*bytes;
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

            let maxp = table_directory.find_table::<MaximumProfile>(bytes)?;
            let header = table_directory.find_table::<FontHeader>(bytes)?;
            metrics = Metrics {
                num_glyphs: maxp.num_glyphs().read_from(bytes)?,
                font_design_units_per_em: header.units_per_em().read_from(bytes)?,
            };
            postscript_name = read_postscript_name(&bytes, table_directory)?;
            cmap = Cmap::parse(bytes, table_directory)?;
        }

        Ok(Font { bytes, postscript_name, metrics, cmap })
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn each_code_point<F>(&self, f: F)-> Result<(), FontError>
        where F: FnMut(char, GlyphId)
    {
        self.cmap.each_code_point(&self.bytes, f)
    }

    pub fn to_glyph_ids(&self, text: &str) -> Result<Vec<GlyphId>, FontError> {
        macro_rules! convert {
            ($get: expr) => {
                text.chars().map(|c| {
                    $get(c as u32).map(|opt| {
                        const NOTDEF_GLYPH: u16 = 0;
                        GlyphId(opt.unwrap_or(NOTDEF_GLYPH))
                    })
                }).collect()
            }
        }
        let bytes = self.bytes();
        match self.cmap {
            Cmap::Format4(ref table) => convert!(|c| table.get(bytes, c)),
            Cmap::Format12(ref table) => convert!(|c| table.get(bytes, c)),
        }
    }
}

fn read_postscript_name(bytes: &[u8], table_directory: Slice<TableDirectoryEntry>)
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

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DebugAsDisplay(&'static str);

        impl fmt::Debug for DebugAsDisplay {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(self.0)
            }
        }

        let Font { bytes: _, ref postscript_name, ref cmap, ref metrics } = *self;
        f.debug_struct("Font")
            .field("bytes", &DebugAsDisplay("[…]"))
            .field("postscript_name", postscript_name)
            .field("cmap", &DebugAsDisplay(match *cmap {
                Cmap::Format4(_) => "Format4(…)",
                Cmap::Format12(_) => "Format12(…)",
            }))
            .field("metrics", metrics)
            .finish()
    }
}
