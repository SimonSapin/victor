// The structs’ fields are not actually used, they are only input to #[derive(SfntTable)]
#![allow(dead_code)]

use crate::fonts::types::*;

#[derive(SfntTable)]
pub(in crate::fonts) struct OffsetSubtable {
    scaler_type: u32,
    table_count: u16,
    _search_range: u16,
    _entry_selector: u16,
    _range_shift: u16,
}

#[derive(SfntTable)]
pub(in crate::fonts) struct TableDirectoryEntry {
    tag: Tag,
    _checksum: u32,
    table_offset: u32,
    _length: u32,
}

#[derive(SfntTable)]
#[tag = "maxp"]
pub(in crate::fonts) struct MaximumProfile {
    _version: FixedPoint,
    num_glyphs: u16,
    // Depending of `version`, this table may have more fields that we don’t use.
    _padding: u16
}

#[derive(SfntTable)]
#[tag = "name"]
pub(in crate::fonts) struct NamingTableHeader {
    _format: u16,
    count: u16,
    string_offset: u16,
}

#[derive(SfntTable)]
pub(in crate::fonts) struct NameRecord {
    platform_id: u16,
    encoding_id: u16,
    _language_id: u16,
    name_id: u16,
    length: u16,
    string_offset: u16,
}

#[derive(SfntTable)]
#[tag = "cmap"]
pub(in crate::fonts) struct CmapHeader {
    _version: u16,
    num_tables: u16,
}

#[derive(SfntTable)]
pub(in crate::fonts) struct CmapEncodingRecord {
    platform_id: u16,
    encoding_id: u16,
    subtable_offset: u32,
}

#[derive(SfntTable)]
pub(in crate::fonts) struct CmapFormat4Header {
    _format: u16,
    _length: u16,
    _language: u16,
    segment_count_x2: u16,
    _search_range: u16,
    _entry_selector: u16,
    _range_shift: u16,
}

#[derive(SfntTable)]
pub(in crate::fonts) struct CmapFormat12Header {
    _format: u16,
    __reserved: u16,
    _length: u32,
    _language: u32,
    num_groups: u32,
}

#[derive(SfntTable)]
pub(in crate::fonts) struct CmapFormat12Group {
    start_char_code: u32,
    end_char_code: u32,
    start_glyph_id: u32,
}


#[derive(SfntTable)]
#[tag = "head"]
pub(in crate::fonts) struct FontHeader {
    _version: FixedPoint,
    _font_revision: FixedPoint,
    _checksum_adjustment: u32,
    _magic_number: u32,
    _flags: u16,
    units_per_em: FontDesignUnitsPerEmFactorU16,
    _created: LongDateTime,
    _modified: LongDateTime,
    min_x: FWord,
    min_y: FWord,
    max_x: FWord,
    max_y: FWord,
    _mac_style: u16,
    _smallest_readable_size_in_px_per_em: u16,
    _font_direction_hint: i16,
    _index_to_loc_format: i16,
    _glyph_data_format: i16,
    __padding: u16,
}

#[derive(SfntTable)]
#[tag = "hhea"]
pub(in crate::fonts) struct HorizontalHeader {
    _version: FixedPoint,
    ascender: FWord,
    descender: FWord,
    _line_gap: FWord,
    _max_advance_width: UFWord,
    _min_left_side_bearing: FWord,
    _max_left_side_bearing: FWord,
    _x_max_extent: FWord,
    _caret_slope_rise: i16,
    _caret_slope_run: i16,
    _carret_offset: FWord,
    __reserved_1: i16,
    __reserved_2: i16,
    __reserved_3: i16,
    __reserved_4: i16,
    _metric_data_format: i16,
    number_of_long_horizontal_metrics: u16,
}

#[derive(SfntTable)]
#[tag = "hmtx"]
pub(in crate::fonts) struct LongHorizontalMetricsRecord {
    advance_width: UFWord,
    _left_side_bearing: i16,
}
