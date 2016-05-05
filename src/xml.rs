use arena::Arena as GenericArena;
use std::cell::Cell;
use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;
use xml_rs::reader::{ParserConfig, EventReader, XmlEvent};

pub use xml_rs::attribute::OwnedAttribute;
pub use xml_rs::name::OwnedName;
pub use xml_rs::reader::{Error, Result};

pub struct Parser<'arena> {
    arena: GenericArena<Node<'arena>>
}

pub struct Node<'arena> {
    parent: Link<'arena>,
    next_sibling: Link<'arena>,
    previous_sibling: Link<'arena>,
    first_child: Link<'arena>,
    last_child: Link<'arena>,
    pub data: NodeData,
}

pub type Ref<'arena> = &'arena Node<'arena>;

type Link<'arena> = Cell<Option<Ref<'arena>>>;

#[derive(Debug)]
pub enum NodeData {
    Document,
    Element(ElementData),
    Text(String),
    ProcessingInstruction {
        name: String,
        data: String
    },
}

#[derive(Debug)]
pub struct ElementData {
    pub name: OwnedName,
    pub attributes: Vec<OwnedAttribute>,
}

impl<'arena> Parser<'arena> {
    pub fn new() -> Self {
        Parser {
            arena: GenericArena::new()
        }
    }

    pub fn parse_file<P: AsRef<Path>>(&'arena self, name: P) -> Result<Ref<'arena>> {
        self.parse(BufReader::new(try!(File::open(name))))
    }

    pub fn parse<R: Read>(&'arena self, stream: R) -> Result<Ref<'arena>> {
        let config = ParserConfig {
            trim_whitespace: false,
            whitespace_to_characters: true,
            cdata_to_characters: true,
            ignore_comments: true,
            coalesce_characters: true,
        };
        let mut parser = config.create_reader(stream);
        let document = self.new_node(NodeData::Document);
        try!(self.parse_content(document, &mut parser));
        Ok(document)
    }

    fn parse_content<R: Read>(&'arena self, parent: Ref<'arena>, parser: &mut EventReader<R>)
                              -> Result<()> {
        loop {
            match try!(parser.next()) {
                XmlEvent::EndDocument | XmlEvent::EndElement { .. } => return Ok(()),

                XmlEvent::StartElement { name, attributes, .. } => {
                    let element = self.append_to(parent, NodeData::Element(ElementData {
                        name: name,
                        attributes: attributes
                    }));
                    try!(self.parse_content(element, parser))
                }

                XmlEvent::ProcessingInstruction { name, data } => {
                    self.append_to(parent, NodeData::ProcessingInstruction{
                        name: name,
                        data: data.unwrap_or_else(String::new),
                    });
                }

                XmlEvent::Characters(text) => {
                    self.append_to(parent, NodeData::Text(text));
                }

                XmlEvent::StartDocument { .. } if matches!(parent.data, NodeData::Document) => {}
                XmlEvent::StartDocument { .. } |
                XmlEvent::CData(_) |
                XmlEvent::Comment(_) |
                XmlEvent::Whitespace(_) => unreachable!()
            }
        }
    }

    fn new_node(&'arena self, data: NodeData) -> Ref<'arena> {
        self.arena.push(Node {
            parent: Cell::new(None),
            previous_sibling: Cell::new(None),
            next_sibling: Cell::new(None),
            first_child: Cell::new(None),
            last_child: Cell::new(None),
            data: data,
        })
    }

    fn append_to(&'arena self, parent: Ref<'arena>, new_child_data: NodeData) -> Ref<'arena> {
        let new_child = self.new_node(new_child_data);
        if let Some(former_last_child) = parent.last_child.get() {
            new_child.previous_sibling.set(Some(former_last_child));
            former_last_child.next_sibling.set(Some(new_child));
        } else {
            debug_assert!(parent.first_child.get().is_none());
            parent.first_child.set(Some(new_child))
        }
        parent.last_child.set(Some(new_child));
        new_child.parent.set(Some(parent));
        new_child
    }
}

macro_rules! link_getters {
    ($($link: ident),+) => {
        $(
            #[inline] pub fn $link(&self) -> Option<Ref<'arena>> { self.$link.get() }
        )+
    }
}

impl<'arena> Node<'arena> {
    link_getters!(parent, previous_sibling, next_sibling, first_child, last_child);

    pub fn iter<F: FnMut(Ref<'arena>)>(&'arena self, callback: &mut F) {
        callback(self);
        let mut link = self.first_child();
        while let Some(node) = link {
            node.iter(callback);
            link = node.next_sibling()
        }
    }

    pub fn element_data(&self) -> Option<&ElementData> {
        match self.data {
            NodeData::Element(ref e) => Some(e),
            _ => None,
        }
    }
}
