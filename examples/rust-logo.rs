#[macro_use] extern crate string_cache;
extern crate victor;

fn main() {
    let parser = victor::xml::Parser::new();
    let doc = parser.parse(include_bytes!("rust-logo/rust-logo-blk.svg").as_ref()).unwrap();
    let selector = victor::SelectorList::parse("svg > path[d]:first-child").unwrap();
    let element = selector.query(doc).unwrap();
    let attribute = element.attribute(&atom!("d")).unwrap();
    let mut path = victor::svg::path::parse(attribute);
    for command in &mut path {
        println!("{:?}", command)
    }
    if let Some(error) = path.error() {
        println!("{:?}", error)
    }
}
