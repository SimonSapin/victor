extern crate opentype;
#[macro_use] extern crate victor;

use opentype::Font;
use std::io::Cursor;
use std::str;
use opentype::truetype::{FontHeader, HorizontalHeader, NamingTable, CharMapping};

static AHEM: victor::fonts::LazyStaticFont = include_font!("../fonts/ahem/ahem.ttf");

fn main() {
    inspect("ahem.ttf", AHEM.bytes());
    inspect("Vera.ttf", victor::fonts::BITSTREAM_VERA_SANS.bytes());
}

fn inspect(name: &str, bytes: &[u8]) {
    println!("\n{}: {} bytes, alignment: {}", name, bytes.len(), (bytes.as_ptr() as usize) % 4);

    let mut cursor = Cursor::new(bytes);
    let font = Font::read(&mut cursor).unwrap();
    macro_rules! take {
        () => { font.take(&mut cursor).unwrap().unwrap() }
    }

    // 'true' (0x74727565) and 0x00010000 mean TrueType
    println!("version: {:08X}", font.offset_table.header.version);

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

    let cmap: CharMapping = take!();
    for encoding in &cmap.encodings {
        println!("cmap length: {}", encoding.mapping().len());
    }
}
