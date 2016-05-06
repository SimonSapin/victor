#[macro_use] extern crate string_cache;
extern crate victor;

fn main() {
    let parser = victor::xml::Parser::new();
    let doc = parser.parse(include_bytes!("rust-logo/rust-logo-blk.svg").as_ref()).unwrap();
    let selector = victor::SelectorList::parse("path[d]").unwrap();
    doc.iter(&mut |node| {
        if let Some(element) = node.as_element() {
            if selector.matches(element) {
                println!("<path>");
                let attribute = element.attribute(&atom!("d")).unwrap();
                let mut path = victor::svg::path::parse(attribute);
                for command in &mut path {
                    println!("    {:?}", command)
                }
                if let Some(error) = path.error() {
                    println!("");
                    println!("    Error around byte {}: {}.", error.position, error.reason);
                }
            }
        }
    })
}
