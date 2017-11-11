use std::borrow::Cow;
use std::char;
use std::collections::BTreeMap;
use std::mem::size_of;
use std::sync::Arc;

mod static_;
mod ttf_tables;
mod ttf_types;

pub use self::static_::*;
use self::ttf_tables::*;
use self::ttf_types::*;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub(crate) struct GlyphId(pub u16);

pub struct Font {
    pub(crate) bytes: Cow<'static, [u8]>,
    pub(crate) postscript_name: String,
    pub(crate) cmap: BTreeMap<char, GlyphId>,
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
    UnsupportedFormat,
}

impl Font {
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Arc<Self>, FontError> {
        Self::from_cow(bytes.into())
    }

    fn from_static(bytes: &'static [u8]) -> Result<Arc<Self>, FontError> {
        Self::from_cow(bytes.into())
    }

    fn from_cow(bytes: Cow<'static, [u8]>) -> Result<Arc<Self>, FontError> {
        let mut font = parse(&bytes)?;
        font.bytes = bytes;
        Ok(Arc::new(font))
    }

    pub fn to_glyph_ids(&self, text: &str) -> Vec<u16> {
        text.chars().map(|c| {
            const NOTDEF_GLYPH: GlyphId = GlyphId(0);
            self.cmap.get(&c).cloned().unwrap_or(NOTDEF_GLYPH).0
        }).collect()
    }
}

fn parse(bytes: &[u8]) -> Result<Font, FontError> {
    let offset_table = OffsetSubtable::cast(bytes, 0);

    let scaler_type = offset_table.scaler_type.value();
    const TRUETYPE: u32 = 0x74727565;  // "true" in big-endian
    if scaler_type != TRUETYPE && scaler_type != 0x_0001_0000 {
        Err(FontError::UnsupportedFormat)?
    }

    let table_count = offset_table.table_count.value() as usize;
    let table_directory_start = size_of::<OffsetSubtable>();
    let table_directory = TableDirectoryEntry::cast_slice(bytes, table_directory_start, table_count);
    let table_offset = |tag: &[u8; 4]| {
        table_directory.iter().find(|e| e.tag == tag).unwrap().offset.value() as usize
    };

    let naming_table_offset = table_offset(b"name");
    let naming_table_header = NamingTableHeader::cast(bytes, naming_table_offset);
    let name_records = NameRecord::cast_slice(
        bytes,
        naming_table_offset.saturating_add(size_of::<NamingTableHeader>()),
        naming_table_header.count.value() as usize,
    );
    let string_storage_start = naming_table_offset
        .saturating_add(naming_table_header.string_offset.value() as usize);
    let get_string_bytes = |offset: &u16_be, length: &u16_be| u8::cast_slice(
        bytes,
        string_storage_start.saturating_add(offset.value() as usize),
        length.value() as usize,
    );
    let decode_macintosh = |offset, length| {
        // Macintosh encodings seem to be ASCII-compatible, and a PostScript name is within ASCII
        String::from_utf8_lossy(get_string_bytes(offset, length)).into_owned()
    };
    let decode_ucs2 = |offset, length| {
        get_string_bytes(offset, length).chunks(2).map(|chunk| {
            if chunk.len() < 2 || chunk[0] != 0 {
                '\u{FFFD}'
            } else {
                chunk[1] as char
            }
        }).collect::<String>()
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
    }).next().unwrap();

    let maximum_profile = MaximumProfile::cast(bytes, table_offset(b"maxp"));
    let glyph_count = maximum_profile.num_glyphs.value() as usize;

    let cmap_offset = table_offset(b"cmap");
    let cmap_header = CmapHeader::cast(bytes, cmap_offset);
    let cmap_records = CmapEncodingRecord::cast_slice(
        bytes,
        cmap_offset.saturating_add(size_of::<CmapHeader>()),
        cmap_header.num_tables.value() as usize,
    );
    // Entries are sorted by (platform, encoding). Reverse to prefer (3, 10) over (3, 1).
    let cmap = cmap_records.iter().rev().filter_map(|record| {
        let offset = cmap_offset.saturating_add(record.offset.value() as usize);
        let format = u16_be::cast(bytes, offset).value();
        const MICROSOFT: u16 = 3;
        const UNICODE_USC2: u16 = 1;
        const UNICODE_USC4: u16 = 10;
        const SEGMENT_MAPPING_TO_DELTA_VALUES: u16 = 4;
        const SEGMENTED_COVERAGE: u16 = 12;
        match (record.platform_id.value(), record.encoding_id.value(), format) {
            (MICROSOFT, UNICODE_USC2, SEGMENT_MAPPING_TO_DELTA_VALUES) => {
                Some(parse_format4_cmap(bytes, offset))
            }
            (MICROSOFT, UNICODE_USC4, SEGMENTED_COVERAGE) => {
                Some(parse_format12_cmap(bytes, offset))
            }
            _ => None,
        }
    }).next().unwrap();

    let header = FontHeader::cast(bytes, table_offset(b"head"));
    let horizontal_header = HorizontalHeader::cast(bytes, table_offset(b"hhea"));

    let ttf_units_per_em = i32::from(header.units_per_em.value());
    const PDF_GLYPH_SPACE_UNITS_PER_EM: i32 = 1000;
    let ttf_to_pdf = |x: i16| i32::from(x) * PDF_GLYPH_SPACE_UNITS_PER_EM / ttf_units_per_em;

    let horizontal_metrics = LongHorizontalMetric::cast_slice(
        bytes,
        table_offset(b"hmtx"),
        horizontal_header.number_of_long_horizontal_metrics.value() as usize,
    );
    let mut glyph_widths = horizontal_metrics
        .iter()
        .map(|m| ttf_to_pdf(m.advance_width.value() as i16) as u16)
        .collect::<Vec<_>>();
    let last_glyph_width = *glyph_widths.last().unwrap();
    glyph_widths.extend(
        (horizontal_metrics.len()..glyph_count as usize)
        .map(|_| last_glyph_width)
    );

    Ok(Font {
        bytes: Cow::Borrowed(&[]),
        min_x: ttf_to_pdf(header.min_x.value()),
        min_y: ttf_to_pdf(header.min_y.value()),
        max_x: ttf_to_pdf(header.max_x.value()),
        max_y: ttf_to_pdf(header.max_y.value()),
        ascent: ttf_to_pdf(horizontal_header.ascender.value()),
        descent: ttf_to_pdf(horizontal_header.descender.value()),
        postscript_name, cmap, glyph_widths,
    })
}

fn parse_format4_cmap(bytes: &[u8], record_offset: usize) -> BTreeMap<char, GlyphId> {
    let encoding_header = CmapFormat4Header::cast(bytes, record_offset);
    let segment_count = encoding_header.segment_count_x2.value() as usize / 2;
    let subtable_size = segment_count.saturating_mul(size_of::<u16>());

    let end_codes_start = record_offset
        .saturating_add(size_of::<CmapFormat4Header>());
    let start_codes_start = end_codes_start
        .saturating_add(subtable_size)  // Add end_code subtable
        .saturating_add(size_of::<u16>());  // Add reserved_padding
    let id_deltas_start = start_codes_start.saturating_add(subtable_size);
    let id_range_offsets_start = id_deltas_start.saturating_add(subtable_size);

    let end_codes = u16_be::cast_slice(bytes, end_codes_start, segment_count);
    let start_codes = u16_be::cast_slice(bytes, start_codes_start, segment_count);
    let id_deltas = u16_be::cast_slice(bytes, id_deltas_start, segment_count);
    let id_range_offsets = u16_be::cast_slice(bytes, id_range_offsets_start, segment_count);

    let mut cmap = BTreeMap::new();
    let iter = end_codes.iter().zip(start_codes).zip(id_deltas).zip(id_range_offsets);
    for (segment_index, (((end_code, start_code), id_delta), id_range_offset)) in iter.enumerate() {
        let end_code: u16 = end_code.value();
        let start_code: u16 = start_code.value();
        let id_delta: u16 = id_delta.value();  // Really i16, but used modulo 2^16 with wrapping_add.
        let id_range_offset: u16 = id_range_offset.value();

        let mut code_point = start_code;
        loop {
            let glyph_id;
            if id_range_offset != 0 {
                let offset =
                    id_range_offsets_start +
                    segment_index * size_of::<u16>() +
                    id_range_offset as usize +
                    (code_point - start_code) as usize * size_of::<u16>();
                let result = u16_be::cast(bytes, offset).value();
                if result != 0 {
                    glyph_id = result.wrapping_add(id_delta)
                } else {
                    glyph_id = 0
                }
            } else {
                glyph_id = code_point.wrapping_add(id_delta)
            };
            if glyph_id != 0 {
                // Ignore any mapping for surrogate code points
                if let Some(ch) = char::from_u32(u32::from(code_point)) {
                    let previous_glyph_id = cmap.insert(ch, GlyphId(glyph_id));
                    assert!(previous_glyph_id.is_none());
                }
            }

            if code_point == end_code {
                break
            }
            code_point += 1;
        }
    }
    cmap
}

fn parse_format12_cmap(bytes: &[u8], record_offset: usize) -> BTreeMap<char, GlyphId> {
    let encoding_header = CmapFormat12Header::cast(bytes, record_offset);
    let groups = CmapFormat12Group::cast_slice(bytes,
        record_offset.saturating_add(size_of::<CmapFormat12Header>()),
        encoding_header.num_groups.value() as usize,
    );

    let mut cmap = BTreeMap::new();
    for group in groups {
        let start_code = group.start_char_code.value();
        let end_code = group.end_char_code.value();
        let start_glyph_id = group.start_glyph_id.value();
        let mut code_point = start_code;
        loop {
            let glyph_id = ((code_point - start_code) + start_glyph_id) as u16;
            if glyph_id != 0 {
                // Ignore any mapping for surrogate code points
                if let Some(ch) = char::from_u32(u32::from(code_point)) {
                    let previous_glyph_id = cmap.insert(ch, GlyphId(glyph_id));
                    assert!(previous_glyph_id.is_none());
                }
            }

            if code_point == end_code {
                break
            }
            code_point += 1;
        }
    }
    cmap
}
