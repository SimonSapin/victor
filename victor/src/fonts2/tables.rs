// The structs’ fields are not actually used, they are only input to #[derive(SfntTable)]
#![allow(dead_code)]

use fonts2::types::*;

#[derive(SfntTable)]
pub(in fonts2) struct OffsetSubtable {
    scaler_type: u32,
    table_count: u16,
    _search_range: u16,
    _entry_selector: u16,
    _range_shift: u16,
}

#[derive(SfntTable)]
pub(in fonts2) struct TableDirectoryEntry {
    tag: Tag,
    _checksum: u32,
    table_offset: u32,
    _length: u32,
}

#[derive(SfntTable)]
#[tag = "maxp"]
pub(in fonts2) struct MaximumProfile {
    _version: FixedPoint,
    num_glyphs: u16,
    // Depending of `version`, this table may have more fields that we don’t use.
    _padding: u16
}

#[derive(SfntTable)]
#[tag = "name"]
pub(in fonts2) struct NamingTableHeader {
    _format: u16,
    count: u16,
    string_offset: u16,
}

#[derive(SfntTable)]
pub(in fonts2) struct NameRecord {
    platform_id: u16,
    encoding_id: u16,
    _language_id: u16,
    name_id: u16,
    length: u16,
    string_offset: u16,
}

#[derive(SfntTable)]
#[tag = "cmap"]
pub(in fonts2) struct CmapHeader {
    _version: u16,
    num_tables: u16,
}

#[derive(SfntTable)]
pub(in fonts2) struct CmapEncodingRecord {
    platform_id: u16,
    encoding_id: u16,
    subtable_offset: u32,
}

#[derive(SfntTable)]
pub(in fonts2) struct CmapFormat4Header {
    _format: u16,
    _length: u16,
    _language: u16,
    segment_count_x2: u16,
    _search_range: u16,
    _entry_selector: u16,
    _range_shift: u16,
}

#[derive(SfntTable)]
pub(in fonts2) struct CmapFormat12Header {
    _format: u16,
    __reserved: u16,
    _length: u32,
    _language: u32,
    num_groups: u32,
}

#[derive(SfntTable)]
pub(in fonts2) struct CmapFormat12Group {
    start_char_code: u32,
    end_char_code: u32,
    start_glyph_id: u32,
}
