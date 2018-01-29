mod cmap;
mod parsing;
mod static_;
mod tables;
mod types;

use euclid;
use fonts::cmap::Cmap;
use fonts::parsing::*;
use fonts::tables::*;
use fonts::types::Tag;
use std::borrow::Cow;
use std::sync::Arc;

pub use fonts::static_::*;

/// The EM square unit
pub(crate) struct Em;

/// The unit of FWord and UFWord
pub(crate) struct FontDesignUnit;


#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct GlyphId(pub u16);

#[derive(Debug)]
pub enum FontError {
    /// Victor only supports TrueType fonts at the moment.
    UnsupportedFormat,

    /// The font file contains an offset to beyond the end of the file.
    OffsetBeyondEof,

    /// The font file contains an offset that puts the end of the pointed object
    /// beyond the end of the file.
    OffsetPlusLengthBeyondEof,

    /// One of the required TrueType tables is missing in this font.
    MissingTable,

    /// This font doesn’t have a “PostScript name” string in a supported encoding.
    NoSupportedPostscriptName,

    /// This font doesn’t have a character map in a supported format.
    NoSupportedCmap,

    /// This font doesn’t have any horizontal metrics for glyphs.
    NoHorizontalGlyphMetrics,
}

pub struct Font {
    bytes: Cow<'static, [u8]>,
    cmap: Cmap,
    pub(crate) postscript_name: String,

    /// Indexed by glyph ID
    pub(crate) glyph_widths: Vec<euclid::Length<u16, FontDesignUnit>>,

    pub(crate) font_design_units_per_em: euclid::TypedScale<u16, Em, FontDesignUnit>,

    /// Distance from baseline of highest ascender
    pub(crate) ascender: euclid::Length<i16, FontDesignUnit>,

    /// Distance from baseline of lowest descender
    pub(crate) descender: euclid::Length<i16, FontDesignUnit>,

    /// The bounding box of the union of all glyphs
    pub(crate) min_x: euclid::Length<i16, FontDesignUnit>,
    pub(crate) min_y: euclid::Length<i16, FontDesignUnit>,
    pub(crate) max_x: euclid::Length<i16, FontDesignUnit>,
    pub(crate) max_y: euclid::Length<i16, FontDesignUnit>,
}

#[cfg(target_pointer_width = "64")]
fn _assert_size_of() {
    let _ = ::std::mem::transmute::<Cmap, [u8; 36]>;
    let _ = ::std::mem::transmute::<Font, [u8; 136]>;
}

impl Font {
    pub fn parse<B: Into<Cow<'static, [u8]>>>(bytes: B) -> Result<Arc<Self>, FontError> {
        Self::parse_cow(bytes.into())
    }

    fn parse_cow(bytes: Cow<'static, [u8]>) -> Result<Arc<Self>, FontError> {
        let mut font = Self::parse_without_cow_bytes_field(&bytes)?;
        font.bytes = bytes;
        Ok(Arc::new(font))
    }

    #[inline]
    fn parse_without_cow_bytes_field(bytes: &[u8]) -> Result<Self, FontError> {
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
        let glyph_count = maxp.num_glyphs().read_from(bytes)?;
        let horizontal_header = table_directory.find_table::<HorizontalHeader>(bytes)?;
        let number_of_long_horizontal_metrics =
            horizontal_header.number_of_long_horizontal_metrics().read_from(bytes)?;
        let horizontal_metrics = Slice::new(
            table_directory.find_table::<LongHorizontalMetricsRecord>(bytes)?,
            number_of_long_horizontal_metrics,
        );
        let mut glyph_widths = horizontal_metrics.into_iter().map(|record| {
            record.advance_width().read_from(bytes)
        }).collect::<Result<Vec<_>, _>>()?;
        let last_glyph_width = *glyph_widths.last().ok_or(FontError::NoHorizontalGlyphMetrics)?;
        glyph_widths.extend(
            (number_of_long_horizontal_metrics..glyph_count)
            .map(|_| last_glyph_width)
        );

        Ok(Font {
            bytes: b""[..].into(),
            postscript_name: read_postscript_name(&bytes, table_directory)?,
            cmap: Cmap::parse(bytes, table_directory)?,
            glyph_widths,
            font_design_units_per_em: header.units_per_em().read_from(bytes)?,
            ascender: horizontal_header.ascender().read_from(bytes)?,
            descender: horizontal_header.descender().read_from(bytes)?,
            min_x: header.min_x().read_from(bytes)?,
            min_y: header.min_y().read_from(bytes)?,
            max_x: header.max_x().read_from(bytes)?,
            max_y: header.max_y().read_from(bytes)?,
        })
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
