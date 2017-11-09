#[macro_use] extern crate victor;
#[macro_use] extern crate victor_internal_derive;

use std::mem;

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
    println!("{} tables", offset_table.table_count.value());
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

unsafe impl Pod for u16 {}
unsafe impl Pod for u32 {}

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
    u32_be = 4,
    u16_be = 2,
    OffsetSubtable = 12,
}
