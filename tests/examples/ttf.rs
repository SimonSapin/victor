#[macro_use] extern crate victor;
#[macro_use] extern crate victor_internal_derive;

use std::fmt::{self, Write};
use std::mem::{self, size_of};
use std::slice;

static AHEM: victor::fonts::LazyStaticFont = include_font!("../fonts/ahem/ahem.ttf");

fn main() {
    inspect("ahem.ttf", AHEM.bytes());
    inspect("Vera.ttf", victor::fonts::BITSTREAM_VERA_SANS.bytes());
}

fn inspect(name: &str, bytes: &[u8]) {
    println!("\n{}: {} bytes", name, bytes.len());

    let offset_table = OffsetSubtable::cast(bytes, 0);

    // 'true' (0x74727565) and 0x00010000 mean TrueType
    println!("version: {:08X}", offset_table.scaler_type.value());

    let table_count = offset_table.table_count.value() as usize;
    let table_directory_start = size_of::<OffsetSubtable>();
    let table_directory = TableDirectoryEntry::cast_slice(bytes, table_directory_start, table_count);

    let tags = table_directory.iter().map(|entry| entry.tag).collect::<Vec<_>>();
    println!("{} tables: {:?}", table_directory.len(), tags);

    let table_offset = |tag: &[u8; 4]| {
        table_directory.iter().find(|e| e.tag == tag).unwrap().offset.value() as usize
    };

    let header = FontHeader::cast(bytes, table_offset(b"head"));
    assert_eq!(header.version.value(), (1, 0));
    assert_eq!(header.magic_number.value(), 0x5F0F3CF5);
    println!("crated: {}", header.created.approximate_year());
    println!("modified: {}", header.modified.approximate_year());
    println!("bounding box: {:?}", [(header.min_x.value(), header.min_y.value()),
                                    (header.max_x.value(), header.max_y.value())]);

    let horizontal_header = HorizontalHeader::cast(bytes, table_offset(b"hhea"));
    println!("ascent: {}", horizontal_header.ascent.value());
    println!("descent: {}", horizontal_header.descent.value());

    let maximum_profile = MaximumProfile::cast(bytes, table_offset(b"maxp"));
    println!("number of glyphs: {}", maximum_profile.num_glyphs.value());

    let horizontal_metrics = LongHorizontalMetric::cast_slice(
        bytes,
        table_offset(b"hmtx"),
        horizontal_header.number_of_long_horizontal_metrics.value() as usize,
    );
    println!("number of horizontal metrics: {}", horizontal_metrics.len());

    let naming_table_offset = table_offset(b"name");
    let naming_table_header = NamingTableHeader::cast(bytes, naming_table_offset);
    println!("Name count: {}", naming_table_header.count.value());
    let name_records = NameRecord::cast_slice(
        bytes,
        naming_table_offset.saturating_add(size_of::<NamingTableHeader>()),
        naming_table_header.count.value() as usize,
    );
    let string_storage_start = naming_table_offset
        .saturating_add(naming_table_header.string_offset.value() as usize);
    let name = name_records.iter().find(|record| {
        const POSTSCRIPT_NAME: u16 = 6;
        const MACINTOSH: u16 = 1;
        record.name_id.value() == POSTSCRIPT_NAME &&
        record.platform_id.value() == MACINTOSH
    }).map(|record| {
        let bytes = u8::cast_slice(
            bytes,
            string_storage_start.saturating_add(record.offset.value() as usize),
            record.length.value() as usize,
        );
        // Macintosh seem to be ASCII-compatible, and a PostScript name is within ASCII
        String::from_utf8_lossy(bytes)
    }).unwrap();
    println!("PostScript name: {:?}", name);
}

/// Plain old data: all bit patterns represent valid values
unsafe trait Pod: Sized {
    fn cast(bytes: &[u8], offset: usize) -> &Self {
        &Self::cast_slice(bytes, offset, 1)[0]
    }

    fn cast_slice(bytes: &[u8], offset: usize, n_items: usize) -> &[Self] {
        let required_alignment = mem::align_of::<Self>();
        assert!(required_alignment <= 4,
                "This type requires more alignment than TrueType promises");

        let bytes = &bytes[offset..];
        assert!((bytes.as_ptr() as usize) % required_alignment == 0);

        let required_len = mem::size_of::<Self>().saturating_mul(n_items);
        assert!(bytes.len() >= required_len);

        let ptr = bytes.as_ptr() as *const Self;
        unsafe {
            slice::from_raw_parts(ptr, n_items)
        }
    }
}

unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for u32 {}
unsafe impl<T: Pod> Pod for [T; 4] {}

macro_rules! big_endian_int_wrappers {
    ($( $Name: ident: $Int: ty; )+) => {
        $(
            /// A big-endian integer
            #[derive(Pod)]
            #[repr(C)]
            struct $Name($Int);

            impl $Name {
                /// Return the value in native-endian
                #[inline]
                fn value(&self) -> $Int {
                    <$Int>::from_be(self.0)
                }
            }
        )+
    }
}

big_endian_int_wrappers! {
    u32_be: u32;
    u16_be: u16;
    i16_be: i16;
}

type FWord = i16_be;
type UFWord = u16_be;

#[repr(C)]
#[derive(Pod)]
struct OffsetSubtable {
    scaler_type: u32_be,
    table_count: u16_be,
    search_range: u16_be,
    entry_selector: u16_be,
    range_shift: u16_be,
}

/// `Fixed` in https://www.microsoft.com/typography/otspec/otff.htm#dataTypes
#[repr(C)]
#[derive(Pod)]
struct FixedPoint {
    integral: u16_be,
    fractional: u16_be,
}

impl FixedPoint {
    fn value(&self) -> (u16, u16) {
        (self.integral.value(), self.fractional.value())
    }
}

#[repr(C)]
#[derive(Pod)]
struct LongDateTime {
    // These two field represent a single i64.
    // We split it in two because TrueType only requires 4-byte alignment.
    upper_bits: u32_be,
    lower_bits: u32_be,
}

impl LongDateTime {
    fn seconds_since_1904_01_01_midnight(&self) -> i64 {
        let upper = (self.upper_bits.value() as u64) << 32;
        let lower = self.lower_bits.value() as u64;
        unsafe {
            mem::transmute::<u64, i64>(upper | lower)
        }
    }

    fn approximate_year(&self) -> i64 {
        (self.seconds_since_1904_01_01_midnight() / 60 / 60 / 24 / 365) + 1904
    }
}

#[derive(Pod, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
struct Tag([u8; 4]);

#[repr(C)]
#[derive(Pod)]
struct TableDirectoryEntry {
    tag: Tag,
    checksum: u32_be,
    offset: u32_be,
    length: u32_be,
}

#[repr(C)]
#[derive(Pod)]
struct FontHeader {
    version: FixedPoint,
    font_revision: FixedPoint,
    checksum_adjustment: u32_be,
    magic_number: u32_be,
    flags: u16_be,
    units_per_em: u16_be,
    created: LongDateTime,
    modified: LongDateTime,
    min_x: FWord,
    min_y: FWord,
    max_x: FWord,
    max_y: FWord,
    mac_style: u16_be,
    smallest_readable_size_in_px_per_em: u16_be,
    font_direction_hint: i16_be,
    index_to_loc_format: i16_be,
    glyph_data_format: i16_be,
}

#[repr(C)]
#[derive(Pod)]
struct HorizontalHeader {
    version: FixedPoint,
    ascent: FWord,
    descent: FWord,
    line_gap: FWord,
    max_advance_width: UFWord,
    min_left_side_bearing: FWord,
    max_left_side_bearing: FWord,
    x_max_extent: FWord,
    caret_slope_rise: i16_be,
    caret_slope_run: i16_be,
    carret_offset: FWord,
    _reserved_1: i16_be,
    _reserved_2: i16_be,
    _reserved_3: i16_be,
    _reserved_4: i16_be,
    metric_data_format: i16_be,
    number_of_long_horizontal_metrics: u16_be,
}

#[repr(C)]
#[derive(Pod)]
struct MaximumProfile {
    version: FixedPoint,
    num_glyphs: u16_be,
    // Depending of `version`, this table may have more fields that we don’t use.
}

#[repr(C)]
#[derive(Pod)]
struct LongHorizontalMetric {
    advance_width: u16_be,
    left_side_bearing: i16_be,
}

#[repr(C)]
#[derive(Pod)]
struct NamingTableHeader {
    format: u16_be,
    count: u16_be,
    string_offset: u16_be,
}

#[repr(C)]
#[derive(Pod)]
struct NameRecord {
    platform_id: u16_be,
    encoding_id: u16_be,
    language_id: u16_be,
    name_id: u16_be,
    length: u16_be,
    offset: u16_be,
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Tag(")?;
        for &byte in &self.0 {
            if b' ' <= byte && byte <= b'~' {
                // ASCII printable or space
                f.write_char(byte as char)?
            } else {
                write!(f, r"\x{:02X}", byte)?
            }
        }
        f.write_char(')')
    }
}

impl<'a> PartialEq<&'a [u8; 4]> for Tag {
    fn eq(&self, other: &&'a [u8; 4]) -> bool {
        self.0 == **other
    }
}

macro_rules! static_assert_size_of {
    ($( $T: ty = $size: expr, )+) => {
        fn _static_assert_size_of() {
            $(
                let _ = mem::transmute::<$T, [u8; $size]>;
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
