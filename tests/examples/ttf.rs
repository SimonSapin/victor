#[macro_use] extern crate victor;

use std::mem;

static AHEM: victor::fonts::LazyStaticFont = include_font!("../fonts/ahem/ahem.ttf");

fn main() {
    inspect("ahem.ttf", AHEM.bytes());
    inspect("Vera.ttf", victor::fonts::BITSTREAM_VERA_SANS.bytes());
}

fn inspect(name: &str, bytes: &[u8]) {
    println!("\n{}: {} bytes", name, bytes.len());

    let header = Header::cast_from(bytes);

    // 'true' (0x74727565) and 0x00010000 mean TrueType
    println!("version: {:08X}", header.version.value());
    println!("{} tables", header.table_count.value());
}

/// Plain old data: all bit patterns represente valid values
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

macro_rules! ints {
    ($( $Name: ident: $Int: ty; )+) => {
        $(
            #[derive(Copy, Clone)]
            struct $Name($Int);

            unsafe impl Pod for $Name {}

            impl $Name {
                fn value(self) -> $Int {
                    <$Int>::from_be(self.0)
                }
            }
        )+
    }
}

ints! {
    BE32: u32;
    BE16: u16;
}

#[repr(C)]
struct Header {
    version: BE32,
    table_count: BE16,
    search_range: BE16,
    entry_selector: BE16,
    range_shift: BE16,
}

unsafe impl Pod for Header {}
