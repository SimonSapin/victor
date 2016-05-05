#[macro_use] extern crate string_cache;
extern crate victor;

fn main() {
    let parser = victor::xml::Parser::new();
    let doc = parser.parse(include_bytes!("rust-logo/rust-logo-blk.svg").as_ref()).unwrap();
    let selector = victor::SelectorList::parse("svg > path[d]:first-child").unwrap();
    doc.iter(&mut |node| if let Some(element) = node.as_element() {
        if selector.matches(element) {
            println!("{}", element.attribute(&atom!("d")).unwrap())
        }
    })
}
