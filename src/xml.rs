use std::fs::File;
use std::io::{self, Read, BufReader};
use std::path::Path;
use string_cache::{Atom, Namespace, QualName};
use xml_rs::reader::{ParserConfig, EventReader, XmlEvent};

pub use xml_rs::attribute::OwnedAttribute;
pub use xml_rs::name::OwnedName;
pub use xml_rs::reader::{Error, Result};

pub struct Document {
    children: Vec<Node>,
}

impl Document {
    pub fn root_element(&self) -> Option<&Element> {
        self.children.iter().filter_map(Node::as_element).next()
    }

    pub fn iter<F>(&self, callback: &mut F) -> io::Result<()>
    where F: FnMut(&Node) -> io::Result<()> {
        for child in &self.children {
            try!(child.iter(callback))
        }
        Ok(())
    }

    pub fn cursor(&self) -> Cursor {
        Cursor {
            element: self.root_element().unwrap(),
            ancestors: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum Node {
    Element(Element),
    Text(String),
    ProcessingInstruction {
        name: String,
        data: String,
    },
}

#[derive(Debug)]
pub struct Element {
    pub name: QualName,
    pub attributes: Vec<(QualName, String)>,
    pub id: Option<Atom>,
    pub classes: Vec<Atom>,

    pub children: Vec<Node>,
}

impl Element {
    pub fn attribute(&self, local_name: &Atom) -> Option<&str> {
        self.attributes.iter()
            .find(|&&(ref name, _)| name.local == *local_name && name.ns == ns!())
            .map(|&(_, ref value)| &**value)
    }
}

fn to_qualname(name: OwnedName) -> QualName {
    let ns = Atom::from(name.namespace.unwrap_or_else(String::new));
    let local = Atom::from(name.local_name);
    QualName::new(Namespace(ns), local)
}

/// https://html.spec.whatwg.org/#space-character
fn space_character(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\u{0C}' | '\r')
}

impl Node {
    pub fn parse_file<P: AsRef<Path>>(name: P) -> Result<Document> {
        Self::parse(BufReader::new(try!(File::open(name))))
    }

    pub fn parse<R: Read>(stream: R) -> Result<Document> {
        let config = ParserConfig {
            trim_whitespace: false,
            whitespace_to_characters: true,
            cdata_to_characters: true,
            ignore_comments: true,
            coalesce_characters: true,
        };
        let mut parser = config.create_reader(stream);
        Ok(Document {
            children: try!(Self::parse_content(&mut parser)),
        })
    }

    fn parse_content<R: Read>(parser: &mut EventReader<R>) -> Result<Vec<Self>> {
        let mut nodes = Vec::new();
        loop {
            match try!(parser.next()) {
                XmlEvent::EndDocument | XmlEvent::EndElement { .. } => return Ok(nodes),

                XmlEvent::StartElement { name, attributes, .. } => {
                    let mut id = None;
                    let mut classes = Vec::new();
                    nodes.push(Node::Element(Element {
                        name: to_qualname(name),
                        attributes: attributes.into_iter().map(|attr| {
                            let name = to_qualname(attr.name);
                            match name {
                                qualname!("", "id") => {
                                    id = Some(Atom::from(&*attr.value))
                                }
                                qualname!("", "class") => {
                                    // https://svgwg.org/svg2-draft/styling.html#ClassAttribute
                                    // set of space-separated tokens
                                    classes = attr.value.split(space_character)
                                                  .filter(|s| !s.is_empty())
                                                  .map(Atom::from)
                                                  .collect()
                                }
                                _ => {}
                            }
                            (name, attr.value)
                        }).collect(),
                        id: id,
                        classes: classes,
                        children: try!(Self::parse_content(parser)),
                    }))
                }

                XmlEvent::ProcessingInstruction { name, data } => {
                    nodes.push(Node::ProcessingInstruction{
                        name: name,
                        data: data.unwrap_or_else(String::new),
                    })
                }

                XmlEvent::Characters(text) => {
                    nodes.push(Node::Text(text))
                }

                XmlEvent::StartDocument { .. } => {}

                XmlEvent::CData(_) |
                XmlEvent::Comment(_) |
                XmlEvent::Whitespace(_) => unreachable!()
            }
        }
    }

    pub fn iter<F>(&self, callback: &mut F) -> io::Result<()>
    where F: FnMut(&Node) -> io::Result<()> {
        try!(callback(self));
        if let Some(element) = self.as_element() {
            for child in &element.children {
                try!(child.iter(callback))
            }
        }
        Ok(())
    }

    pub fn as_element(&self) -> Option<&Element> {
        match *self {
            Node::Element(ref element) => Some(element),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct Cursor<'document> {
    element: &'document Element,
    ancestors: Vec<(&'document Element, usize)>
}

impl<'document> Cursor<'document> {
    pub fn element(&self) -> &'document Element {
        self.element
    }

    pub fn is_root(&self) -> bool {
        self.ancestors.is_empty()
    }

    pub fn parent_element(&mut self) -> bool {
        if let Some((parent, _)) = self.ancestors.pop() {
            self.element = parent;
            true
        } else {
            false
        }
    }

    pub fn first_child_element(&mut self) -> bool {
        for (i, node) in self.element.children.iter().enumerate() {
            if let Some(element) = node.as_element() {
                self.ancestors.push((self.element, i));
                self.element = element;
                return true
            }
        }
        false
    }

    pub fn last_child_element(&mut self) -> bool {
        for (i, node) in self.element.children.iter().rev().enumerate() {
            if let Some(element) = node.as_element() {
                self.ancestors.push((self.element, i));
                self.element = element;
                return true
            }
        }
        false
    }

    pub fn next_sibling_element(&mut self) -> bool {
        if let Some(&mut (parent, ref mut position)) = self.ancestors.last_mut() {
            let mut new_position = *position + 1;
            for child in &parent.children[new_position..] {
                if let Some(element) = child.as_element() {
                    *position = new_position;
                    self.element = element;
                    return true
                }
                new_position += 1;
            }
        }
        false
    }

    pub fn prev_sibling_element(&mut self) -> bool {
        if let Some(&mut (parent, ref mut position)) = self.ancestors.last_mut() {
            let mut new_position = *position;
            for child in parent.children[..new_position].iter().rev() {
                new_position -= 1;
                if let Some(element) = child.as_element() {
                    *position = new_position;
                    self.element = element;
                    return true
                }
            }
        }
        false
    }

    pub fn next_in_tree_order(&mut self) -> bool {
        if self.first_child_element() {
            return true
        }
        loop {
            if self.next_sibling_element() {
                return true
            }
            if !self.parent_element() {
                return false
            }
        }
    }
}
