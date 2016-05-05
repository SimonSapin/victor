extern crate victor;

fn main() {
    let parser = victor::xml::Parser::new();
    let doc = parser.parse(include_bytes!("rust-logo/rust-logo-blk.svg").as_ref()).unwrap();
    doc.iter(&mut |node| if let Some(element) = node.element_data() {
        print!("<{}", element.name.local_name);
        for attribute in &element.attributes {
            print!(" {}=\"{}\"", attribute.name.local_name, attribute.value)
        }
        println!(">");
    })
}
