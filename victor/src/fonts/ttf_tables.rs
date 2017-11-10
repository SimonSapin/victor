use super::ttf_types::*;

#[repr(C)]
#[derive(Pod)]
pub(crate) struct OffsetSubtable {
    pub scaler_type: u32_be,
    pub table_count: u16_be,
    pub search_range: u16_be,
    pub entry_selector: u16_be,
    pub range_shift: u16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct TableDirectoryEntry {
    pub tag: Tag,
    pub checksum: u32_be,
    pub offset: u32_be,
    pub length: u32_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct FontHeader {
    pub version: FixedPoint,
    pub font_revision: FixedPoint,
    pub checksum_adjustment: u32_be,
    pub magic_number: u32_be,
    pub flags: u16_be,
    pub units_per_em: u16_be,
    pub created: LongDateTime,
    pub modified: LongDateTime,
    pub min_x: FWord,
    pub min_y: FWord,
    pub max_x: FWord,
    pub max_y: FWord,
    pub mac_style: u16_be,
    pub smallest_readable_size_in_px_per_em: u16_be,
    pub font_direction_hint: i16_be,
    pub index_to_loc_format: i16_be,
    pub glyph_data_format: i16_be,
    pub _padding: u16,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct HorizontalHeader {
    pub version: FixedPoint,
    pub ascender: FWord,
    pub descender: FWord,
    pub line_gap: FWord,
    pub max_advance_width: UFWord,
    pub min_left_side_bearing: FWord,
    pub max_left_side_bearing: FWord,
    pub x_max_extent: FWord,
    pub caret_slope_rise: i16_be,
    pub caret_slope_run: i16_be,
    pub carret_offset: FWord,
    pub _reserved_1: i16_be,
    pub _reserved_2: i16_be,
    pub _reserved_3: i16_be,
    pub _reserved_4: i16_be,
    pub metric_data_format: i16_be,
    pub number_of_long_horizontal_metrics: u16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct MaximumProfile {
    pub version: FixedPoint,
    pub num_glyphs: u16_be,
    // Depending of `version`, this table may have more fields that we don’t use.
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct LongHorizontalMetric {
    pub advance_width: u16_be,
    pub left_side_bearing: i16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct NamingTableHeader {
    pub format: u16_be,
    pub count: u16_be,
    pub string_offset: u16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct NameRecord {
    pub platform_id: u16_be,
    pub encoding_id: u16_be,
    pub language_id: u16_be,
    pub name_id: u16_be,
    pub length: u16_be,
    pub offset: u16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct CmapHeader {
    pub version: u16_be,
    pub num_tables: u16_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct CmapEncodingRecord {
    pub platform_id: u16_be,
    pub encoding_id: u16_be,
    pub offset: u32_be,
}

#[repr(C)]
#[derive(Pod)]
pub(crate) struct CmapFormat4Header {
    pub format: u16_be,
    pub length: u16_be,
    pub language: u16_be,
    pub segment_count_x2: u16_be,
    pub search_range: u16_be,
    pub entry_selector: u16_be,
    pub range_shift: u16_be,
}

macro_rules! static_assert_size_of {
    ($( $T: ty = $size: expr, )+) => {
        fn _static_assert_size_of() {
            $(
                let _ = ::std::mem::transmute::<$T, [u8; $size]>;
            )+
        }
    }
}

static_assert_size_of! {
    u16_be = 2,
    u32_be = 4,
    Tag = 4,
    OffsetSubtable = 12,
    TableDirectoryEntry = 16,
}
