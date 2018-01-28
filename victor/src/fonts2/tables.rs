// The structs’ fields are not actually used, they are only input to #[derive(SfntTable)]
#![allow(dead_code)]

use fonts2::parsing::Position;
use fonts2::{SfntTable, Tag};

type FixedPoint = u32;

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
