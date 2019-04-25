mod cmap;
mod parsing;
mod tables;
mod types;

use crate::fonts::cmap::Cmap;
use crate::fonts::parsing::*;
use crate::fonts::tables::*;
use crate::fonts::types::Tag;
use std::borrow::Cow;
use std::cmp;
use std::sync::Arc;

/// The EM square unit
pub(crate) struct Em;

/// The unit of FWord and UFWord
struct FontDesignUnit;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub(crate) struct GlyphId(pub(crate) u16);

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
    postscript_name: String,
    glyph_count: u16,
    font_design_units_per_em: euclid::TypedScale<f32, Em, FontDesignUnit>,
    horizontal_metrics: Slice<LongHorizontalMetricsRecord>,

    /// Distance from baseline of highest ascender
    ascender: euclid::Length<i16, FontDesignUnit>,

    /// Distance from baseline of lowest descender
    descender: euclid::Length<i16, FontDesignUnit>,

    /// The bounding box of the union of all glyphs
    min_x: euclid::Length<i16, FontDesignUnit>,
    min_y: euclid::Length<i16, FontDesignUnit>,
    max_x: euclid::Length<i16, FontDesignUnit>,
    max_y: euclid::Length<i16, FontDesignUnit>,
}

#[cfg(target_pointer_width = "64")]
fn _assert_size_of() {
    let _ = std::mem::transmute::<Cmap, [u8; 24]>;
    let _ = std::mem::transmute::<Font, [u8; 112]>;
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
        const TRUETYPE: u32 = 0x74727565; // "true" in big-endian
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

        Ok(Font {
            bytes: b""[..].into(),
            postscript_name: read_postscript_name(&bytes, table_directory)?,
            cmap: Cmap::parse(bytes, table_directory)?,
            glyph_count,
            horizontal_metrics: Slice::new(
                table_directory.find_table::<LongHorizontalMetricsRecord>(bytes)?,
                horizontal_header
                    .number_of_long_horizontal_metrics()
                    .read_from(bytes)?,
            ),
            font_design_units_per_em: header.units_per_em().read_from(bytes)?.cast(),
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
    pub(crate) fn postscript_name(&self) -> &str {
        &self.postscript_name
    }
    pub(crate) fn glyph_count(&self) -> u16 {
        self.glyph_count
    }

    pub(crate) fn each_code_point<F>(&self, f: F) -> Result<(), FontError>
    where
        F: FnMut(char, GlyphId),
    {
        self.cmap.each_code_point(&self.bytes, f)
    }

    pub(crate) fn glyph_id(&self, ch: char) -> Result<GlyphId, FontError> {
        let ch = ch as u32;
        let result = match self.cmap {
            Cmap::Format4(ref table) => table.get(&self.bytes, ch),
            Cmap::Format12(ref table) => table.get(&self.bytes, ch),
        };
        const NOTDEF_GLYPH: u16 = 0;
        Ok(GlyphId(result?.unwrap_or(NOTDEF_GLYPH)))
    }

    pub(crate) fn glyph_width(
        &self,
        glyph_id: GlyphId,
    ) -> Result<euclid::Length<f32, Em>, FontError> {
        let last_index = self
            .horizontal_metrics
            .count()
            .checked_sub(1)
            .ok_or(FontError::NoHorizontalGlyphMetrics)?;
        let index = cmp::min(glyph_id.0 as u32, last_index);
        let w = self
            .horizontal_metrics
            .get_unchecked(index)
            .advance_width()
            .read_from(&self.bytes)?;
        Ok(self.to_ems(w))
    }

    fn to_ems<T>(&self, length: euclid::Length<T, FontDesignUnit>) -> euclid::Length<f32, Em>
    where
        T: num_traits::NumCast + Clone,
    {
        length.cast() / self.font_design_units_per_em
    }

    pub(crate) fn ascender(&self) -> euclid::Length<f32, Em> {
        self.to_ems(self.ascender)
    }
    pub(crate) fn descender(&self) -> euclid::Length<f32, Em> {
        self.to_ems(self.descender)
    }
    pub(crate) fn min_x(&self) -> euclid::Length<f32, Em> {
        self.to_ems(self.min_x)
    }
    pub(crate) fn min_y(&self) -> euclid::Length<f32, Em> {
        self.to_ems(self.min_y)
    }
    pub(crate) fn max_x(&self) -> euclid::Length<f32, Em> {
        self.to_ems(self.max_x)
    }
    pub(crate) fn max_y(&self) -> euclid::Length<f32, Em> {
        self.to_ems(self.max_y)
    }
}

fn read_postscript_name(
    bytes: &[u8],
    table_directory: Slice<TableDirectoryEntry>,
) -> Result<String, FontError> {
    /// Macintosh encodings seem to be ASCII-compatible, and a PostScript name is within ASCII
    fn decode_macintosh(string_bytes: &[u8]) -> String {
        String::from_utf8_lossy(string_bytes).into_owned()
    }

    /// Latin-1 range only
    fn decode_ucs2(string_bytes: &[u8]) -> String {
        string_bytes
            .chunks(2)
            .map(|chunk| {
                if chunk.len() < 2 || chunk[0] != 0 {
                    '\u{FFFD}'
                } else {
                    chunk[1] as char
                }
            })
            .collect::<String>()
    };

    let naming_table_header = table_directory.find_table::<NamingTableHeader>(bytes)?;
    let name_records = Slice::new(
        naming_table_header.followed_by::<NameRecord>(),
        naming_table_header.count().read_from(bytes)?,
    );
    let string_storage_start: Position<()> =
        naming_table_header.offset_bytes(naming_table_header.string_offset().read_from(bytes)?);
    let string_bytes = |record: Position<NameRecord>| {
        Slice::<u8>::new(
            string_storage_start.offset_bytes(record.string_offset().read_from(bytes)?),
            record.length().read_from(bytes)?,
        )
        .read_from(bytes)
    };

    for record in name_records {
        const POSTSCRIPT_NAME: u16 = 6;
        if record.name_id().read_from(bytes)? != POSTSCRIPT_NAME {
            continue;
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
        return Ok(postscript_name);
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
        Ok(Position::<OffsetSubtable>::initial().offset_bytes(offset))
    }
}

#[doc(hidden)]
pub mod _reexports_for_macros {
    pub use lazy_static::*;
    pub use std::sync::Arc;
}

#[macro_export]
macro_rules! include_fonts {
    ( $( $NAME: ident: $filename: expr, )+ ) => {
        $(
            $crate::fonts::_reexports_for_macros::lazy_static! {
                pub static ref $NAME:
                    $crate::fonts::_reexports_for_macros::Arc<$crate::fonts::Font> =
                {
                    $crate::fonts::Font::parse(
                        include_bytes!($filename) as &'static [u8]
                    ).unwrap()
                };
            }
        )+
    };
}

include_fonts! {
    BITSTREAM_VERA_SANS: "../../fonts/vera/Vera.ttf",
}
