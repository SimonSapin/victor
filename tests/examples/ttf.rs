#[macro_use] extern crate victor;
#[macro_use] extern crate victor_internal_derive;

use std::mem;
use std::fmt::{self, Write};

static AHEM: victor::fonts::LazyStaticFont = include_font!("../fonts/ahem/ahem.ttf");

fn main() {
    inspect("ahem.ttf", AHEM.bytes());
    inspect("Vera.ttf", victor::fonts::BITSTREAM_VERA_SANS.bytes());
}

fn inspect(name: &str, bytes: &[u8]) {
    println!("\n{}: {} bytes", name, bytes.len());

    let offset_table = OffsetSubtable::cast_from(bytes);

    // 'true' (0x74727565) and 0x00010000 mean TrueType
    println!("version: {:08X}", offset_table.scaler_type.value());

    let table_directory_start = mem::size_of::<OffsetSubtable>();
    let table_count = offset_table.table_count.value();
    let table_directory_entries = || {
        (0..table_count as usize).map(|i| {
            let offset = table_directory_start + i * mem::size_of::<TableDirectoryEntry>();
            TableDirectoryEntry::cast_from(&bytes[offset..])
        })
    };

    let tags = table_directory_entries().map(|entry| entry.tag).collect::<Vec<_>>();
    println!("{} tables: {:?}", table_count, tags);

    let table_bytes = |tag: Tag| table_directory_entries().find(|e| e.tag == tag).map(|entry| {
        &bytes[entry.offset.value() as usize..][..entry.length.value() as usize]
    });

    let info = GeneralInformationTable::cast_from(table_bytes(Tag::from(b"head")).unwrap());
}

/// Plain old data: all bit patterns represent valid values
unsafe trait Pod: Sized {
    fn cast_from(bytes: &[u8]) -> &Self {
        assert!((bytes.as_ptr() as usize) % mem::align_of::<Self>() == 0);
        assert!(bytes.len() >= mem::size_of::<Self>());
        let ptr = bytes.as_ptr() as *const Self;
        unsafe {
            &*ptr
        }
    }
}

unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
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
}

#[repr(C)]
#[derive(Pod)]
struct OffsetSubtable {
    scaler_type: u32_be,
    table_count: u16_be,
    search_range: u16_be,
    entry_selector: u16_be,
    range_shift: u16_be,
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
struct GeneralInformationTable {
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

impl From<&'static [u8; 4]> for Tag {
    fn from(bytes_literal: &'static [u8; 4]) -> Self {
        Tag(*bytes_literal)
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
