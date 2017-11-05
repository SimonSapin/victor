extern crate opentype;
extern crate truetype;

use opentype::Font;
use std::io::Cursor;
use truetype::{FontHeader, HorizontalHeader, NamingTable};

fn main() {
    inspect("ahem.ttf", include_bytes!("../../fonts/Ahem-2017.01.31/ahem.ttf"));
    inspect("Vera.ttf", include_bytes!("../../fonts/ttf-bitstream-vera-1.10/Vera.ttf"));
}

fn inspect(name: &str, bytes: &[u8]) {
    println!("\n{}: {} bytes", name, bytes.len());

    let mut cursor = Cursor::new(bytes);
    let font = Font::read(&mut cursor).unwrap();

    let font_header: FontHeader = font.take(&mut cursor).unwrap().unwrap();
    let horizontal_header: HorizontalHeader = font.take(&mut cursor).unwrap().unwrap();

    println!("Units per em: {}", font_header.units_per_em);
    println!("Ascender: {}", horizontal_header.ascender);

    let naming_table: NamingTable = font.take(&mut cursor).unwrap().unwrap();
    match naming_table {
        NamingTable::Format0(ref table) => {
            let strings = table.strings().unwrap();
            for id in (1..10).chain(11..13) {
                println!("Naming table string #{}: {:?}", id, strings[id]);
            }
        },
        _ => unreachable!(),
    }
}
