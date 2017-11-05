extern crate opentype;
extern crate truetype;

use opentype::Font;
use std::io::Cursor;
use std::str;
use truetype::{FontHeader, HorizontalHeader, NamingTable, CharMapping};

macro_rules! include_u32_aligned_bytes {
    ( $filename: expr ) => {{
        #[repr(C)] struct U32Aligned<T>([u32; 0], T);  // T == [u8; $size]
        &U32Aligned([], *include_bytes!($filename)).1
    }}
}

fn main() {
    inspect("ahem.ttf", include_u32_aligned_bytes!("../../fonts/Ahem-2017.01.31/ahem.ttf"));
    inspect("Vera.ttf", include_u32_aligned_bytes!("../../fonts/ttf-bitstream-vera-1.10/Vera.ttf"));
}

fn inspect(name: &str, bytes: &[u8]) {
    assert!((bytes.as_ptr() as usize) % 4 == 0);
    println!("\n{}: {} bytes", name, bytes.len());

    let mut cursor = Cursor::new(bytes);
    let font = Font::read(&mut cursor).unwrap();
    macro_rules! take {
        () => { font.take(&mut cursor).unwrap().unwrap() }
    }

    println!("{} tables: {}", font.offset_table.records.len(),
             font.offset_table.records.iter()
             .map(|r| str::from_utf8(&*r.tag).unwrap())
             .collect::<Vec<_>>()
             .join(", "));

    let font_header: FontHeader = take!();
    let horizontal_header: HorizontalHeader = take!();

    println!("Units per em: {}", font_header.units_per_em);
    println!("Ascender: {}", horizontal_header.ascender);

    let naming_table: NamingTable = take!();
    match naming_table {
        NamingTable::Format0(ref table) => {
            let strings = table.strings().unwrap();
            for &id in &[1, 9, 11] {
                println!("Naming table string #{}: {:?}", id, strings[id]);
            }
        },
        _ => unreachable!(),
    }

    if name == "ahem.ttf" {
        let cmap: CharMapping = take!();
        for encoding in &cmap.encodings {
            println!("cmap length: {}", encoding.mapping().len());
        }
    }
}
