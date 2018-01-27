// The structs’ fields are not actually used, they are only input to #[derive(SfntTable)]
#![allow(dead_code)]

use fonts2::parsing::Position;
use fonts2::Tag;

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
    _offset: u32,
    _length: u32,
}
