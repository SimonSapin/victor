use std::borrow::Cow;
use std::mem::size_of;
use std::sync::Arc;

mod cmap;
mod static_;
mod ttf_tables;
mod ttf_types;

pub use self::static_::*;
use self::ttf_tables::*;
use self::ttf_types::*;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct GlyphId(pub u16);

pub struct Font {
    pub(crate) bytes: AlignedCowBytes,
    pub(crate) postscript_name: String,
    pub(crate) cmap: cmap::Cmap,
    pub(crate) min_x: i32,
    pub(crate) min_y: i32,
    pub(crate) max_x: i32,
    pub(crate) max_y: i32,
    pub(crate) ascent: i32,
    pub(crate) descent: i32,
    pub(crate) glyph_widths: Vec<u16>,
}

#[derive(Debug)]
pub enum FontError {
    /// Victor only supports TrueType fonts at the moment.
    UnsupportedFormat,

    /// Victor’s font parser requires its input to be 32-bit aligned.
    /// Normally the heap allocator used by Vec always aligns to at least twice the pointer size,
    /// but that’s not the case here.
    /// Please file a bug.
    UnalignedVec,

    /// Victor’s font parser requires its input to be 32-bit aligned.
    /// The include_font!() macro normally forces static byte arrays to be aligned,
    /// but that apparently didn’t work.
    /// Please file a bug.
    UnalignedStaticArray,

    /// The font file contains an offset to beyond the end of the file.
    OffsetBeyondEof,

    /// The font file contains an offset that puts the end of the pointed object
    /// beyond the end of the file.
    OffsetPlusLengthBeyondEof,

    /// The font file contains an offset not aligned sufficently for the targeted object.
    UnalignedOffset,

    /// One of the required TrueType tables is missing in this font.
    MissingTable,

    /// This font doesn’t have a “PostScript name” string in a supported encoding.
    NoSupportedPostscriptName,

    /// This font doesn’t have a character map in a supported format.
    NoSupportedCmap,

    /// This font doesn’t have any horizontal metrics for glyphs.
    NoHorizontalGlyphMetrics,
}

impl Font {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Arc<Self>, FontError> {
        Self::from_cow(bytes.into(), FontError::UnalignedVec)
    }

    fn from_cow(bytes: Cow<'static, [u8]>, on_unaligned: FontError) -> Result<Arc<Self>, FontError> {
        let bytes = AlignedCowBytes::new(bytes).ok_or(on_unaligned)?;
        let mut font = parse(bytes.borrow())?;
        font.bytes = bytes;
        Ok(Arc::new(font))
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes.0
    }

    pub(crate) fn each_code_point<F>(&self, f: F)-> Result<(), FontError>
        where F: FnMut(char, GlyphId)
    {
        self.cmap.each_code_point(self.bytes.borrow(), f)
    }

    pub fn to_glyph_ids(&self, text: &str) -> Result<Vec<GlyphId>, FontError> {
        const NOTDEF_GLYPH: u16 = 0;
        match self.cmap {
            cmap::Cmap::Format4 { offset } => {
                let cmap = cmap::Format4::parse(self.bytes.borrow(), offset)?;
                text.chars().map(|c| {
                    cmap.get(c as u32)  // Result<Option<_>, _>
                        .map(|opt| GlyphId(opt.unwrap_or(NOTDEF_GLYPH)))
                }).collect()
            }
            cmap::Cmap::Format12 { offset } => {
                let cmap = cmap::Format12::parse(self.bytes.borrow(), offset)?;
                Ok(text.chars().map(|c| GlyphId(
                    cmap.get(c as u32) // Option
                        .unwrap_or(NOTDEF_GLYPH)
                )).collect())
            }
        }
    }
}

fn parse(bytes: AlignedBytes) -> Result<Font, FontError> {
    let offset_table = OffsetSubtable::cast(bytes, 0)?;

    let scaler_type = offset_table.scaler_type.value();
    const TRUETYPE: u32 = 0x74727565;  // "true" in big-endian
    if scaler_type != TRUETYPE && scaler_type != 0x_0001_0000 {
        Err(FontError::UnsupportedFormat)?
    }

    let table_count = offset_table.table_count.value() as usize;
    let table_directory_start = size_of::<OffsetSubtable>();
    let table_directory = TableDirectoryEntry::cast_slice(bytes, table_directory_start, table_count)?;
    let table_offset = |tag: &[u8; 4]| {
        let index = table_directory
            .binary_search_by_key(tag, |entry| entry.tag.0)
            .map_err(|_| FontError::MissingTable)?;
        Ok(table_directory[index].offset.value() as usize)
    };

    let naming_table_offset = table_offset(b"name")?;
    let naming_table_header = NamingTableHeader::cast(bytes, naming_table_offset)?;
    let name_records = NameRecord::cast_slice(
        bytes,
        naming_table_offset.saturating_add(size_of::<NamingTableHeader>()),
        naming_table_header.count.value() as usize,
    )?;
    let string_storage_start = naming_table_offset
        .saturating_add(naming_table_header.string_offset.value() as usize);
    let get_string_bytes = |offset: &u16_be, length: &u16_be| u8::cast_slice(
        bytes,
        string_storage_start.saturating_add(offset.value() as usize),
        length.value() as usize,
    );
    let decode_macintosh = |offset, length| {
        // Macintosh encodings seem to be ASCII-compatible, and a PostScript name is within ASCII
        Ok(String::from_utf8_lossy(get_string_bytes(offset, length)?).into_owned())
    };
    let decode_ucs2 = |offset, length| {
        Ok(get_string_bytes(offset, length)?.chunks(2).map(|chunk| {
            if chunk.len() < 2 || chunk[0] != 0 {
                '\u{FFFD}'
            } else {
                chunk[1] as char
            }
        }).collect::<String>())
    };

    let postscript_name = name_records.iter().filter(|record| {
        const POSTSCRIPT_NAME: u16 = 6;
        record.name_id.value() == POSTSCRIPT_NAME
    }).filter_map(|record| {
        const MACINTOSH: u16 = 1;
        const MICROSOFT: u16 = 3;
        const UNICODE_BMP: u16 = 1;
        match (record.platform_id.value(), record.encoding_id.value()) {
            (MACINTOSH, _) => Some(decode_macintosh(&record.offset, &record.length)),
            (MICROSOFT, UNICODE_BMP) => Some(decode_ucs2(&record.offset, &record.length)),
            _ => None,
        }
    }).next().ok_or(FontError::NoSupportedPostscriptName)??;

    let maximum_profile = MaximumProfile::cast(bytes, table_offset(b"maxp")?)?;
    let glyph_count = maximum_profile.num_glyphs.value() as usize;

    let cmap_offset = table_offset(b"cmap")?;
    let cmap_header = CmapHeader::cast(bytes, cmap_offset)?;
    let cmap_records = CmapEncodingRecord::cast_slice(
        bytes,
        cmap_offset.saturating_add(size_of::<CmapHeader>()),
        cmap_header.num_tables.value() as usize,
    )?;
    // Entries are sorted by (platform, encoding). Reverse to prefer (3, 10) over (3, 1).
    let cmap = cmap_records.iter().rev().filter_map(|record| {
        let offset = cmap_offset.saturating_add(record.offset.value() as usize);
        let format = match u16_be::cast(bytes, offset) {
            Ok(f) => f.value(),
            Err(e) => return Some(Err(e)),
        };
        const MICROSOFT: u16 = 3;
        const UNICODE_USC2: u16 = 1;
        const UNICODE_USC4: u16 = 10;
        const SEGMENT_MAPPING_TO_DELTA_VALUES: u16 = 4;
        const SEGMENTED_COVERAGE: u16 = 12;
        match (record.platform_id.value(), record.encoding_id.value(), format) {
            (MICROSOFT, UNICODE_USC2, SEGMENT_MAPPING_TO_DELTA_VALUES) => {
                Some(Ok(cmap::Cmap::Format4 { offset }))
            }
            (MICROSOFT, UNICODE_USC4, SEGMENTED_COVERAGE) => {
                Some(Ok(cmap::Cmap::Format12 { offset }))
            }
            _ => None,
        }
    }).next().ok_or(FontError::NoSupportedCmap)??;

    let header = FontHeader::cast(bytes, table_offset(b"head")?)?;
    let horizontal_header = HorizontalHeader::cast(bytes, table_offset(b"hhea")?)?;

    let ttf_units_per_em = i32::from(header.units_per_em.value());
    const PDF_GLYPH_SPACE_UNITS_PER_EM: i32 = 1000;
    let ttf_to_pdf = |x: i16| i32::from(x) * PDF_GLYPH_SPACE_UNITS_PER_EM / ttf_units_per_em;

    let horizontal_metrics = LongHorizontalMetric::cast_slice(
        bytes,
        table_offset(b"hmtx")?,
        horizontal_header.number_of_long_horizontal_metrics.value() as usize,
    )?;
    let mut glyph_widths = horizontal_metrics
        .iter()
        .map(|m| ttf_to_pdf(m.advance_width.value() as i16) as u16)
        .collect::<Vec<_>>();
    let last_glyph_width = *glyph_widths.last().ok_or(FontError::NoHorizontalGlyphMetrics)?;
    glyph_widths.extend(
        (horizontal_metrics.len()..glyph_count as usize)
        .map(|_| last_glyph_width)
    );

    Ok(Font {
        bytes: AlignedCowBytes::empty(),
        min_x: ttf_to_pdf(header.min_x.value()),
        min_y: ttf_to_pdf(header.min_y.value()),
        max_x: ttf_to_pdf(header.max_x.value()),
        max_y: ttf_to_pdf(header.max_y.value()),
        ascent: ttf_to_pdf(horizontal_header.ascender.value()),
        descent: ttf_to_pdf(horizontal_header.descender.value()),
        postscript_name, cmap, glyph_widths,
    })
}
