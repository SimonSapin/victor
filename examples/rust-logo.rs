#[macro_use] extern crate string_cache;
extern crate victor;

fn main() {
    let parser = victor::xml::Parser::new();
    let doc = parser.parse(include_bytes!("rust-logo/rust-logo-blk.svg").as_ref()).unwrap();
    let selector = victor::SelectorList::parse("svg > path[d]:first-child").unwrap();
    let path = selector.query(doc).unwrap();
    println!("{}", path.attribute(&atom!("d")).unwrap())
}
