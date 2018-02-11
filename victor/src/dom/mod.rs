//! This is *a* Document Object Model, but is not necessarily compatible with *the* DOM.

mod html;

use arena::Arena;
use html5ever::{QualName, ExpandedName, LocalName, Attribute};
use html5ever::tendril::StrTendril;
use std::cell::{Cell, RefCell, Ref};
use std::fmt;
use std::ptr;
use style::StyleSet;

pub type ArenaRef<'arena> = &'arena Arena<Node<'arena>>;
pub(crate) type NodeRef<'arena> = &'arena Node<'arena>;
pub(crate) type Link<'arena> = Cell<Option<NodeRef<'arena>>>;

pub struct Document<'arena> {
    document_node: NodeRef<'arena>,
    style_elements: Vec<NodeRef<'arena>>
}

impl<'arena> Document<'arena> {
    pub fn parse_stylesheets(&self, style_set: &mut StyleSet) {
        for element in &self.style_elements {
            // https://html.spec.whatwg.org/multipage/semantics.html#update-a-style-block
            if let Some(type_attr) = element.as_element().unwrap().get_attr(&local_name!("type")) {
                if !type_attr.eq_ignore_ascii_case("text/css") {
                    continue
                }
            }
            style_set.add_stylesheet(&element.child_text_content())
        }
    }
}

pub struct Node<'arena> {
    pub(crate) parent: Link<'arena>,
    pub(crate) next_sibling: Link<'arena>,
    pub(crate) previous_sibling: Link<'arena>,
    pub(crate) first_child: Link<'arena>,
    pub(crate) last_child: Link<'arena>,
    data: NodeData,
}

enum NodeData {
    Document,
    Doctype {
        _name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    },
    Text {
        contents: RefCell<StrTendril>,
    },
    Comment {
        _contents: StrTendril,
    },
    Element(ElementData),
    ProcessingInstruction {
        _target: StrTendril,
        _contents: StrTendril,
    },
}

pub(crate) struct ElementData {
    pub(crate) name: QualName,
    pub(crate) attrs: RefCell<Vec<Attribute>>,
    pub(crate) mathml_annotation_xml_integration_point: bool,
}

impl ElementData {
    pub(crate) fn get_attr(&self, name: &LocalName) -> Option<Ref<str>>{
        let name = ExpandedName { ns: &ns!(), local: name };
        let attrs = self.attrs.borrow();
        attrs.iter().position(|attr| attr.name.expanded() == name).map(|i| {
            Ref::map(attrs, |attrs| &*attrs[i].value)
        })
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn size_of() {
    use std::mem::size_of;
    assert_eq!(size_of::<Node>(), 120);
    assert_eq!(size_of::<NodeData>(), 80);
    assert_eq!(size_of::<ElementData>(), 72);
}

impl<'arena> Node<'arena> {
    pub(crate) fn in_html_document(&self) -> bool {
        // FIXME: track something when we add XML parsing
        true
    }

    pub(crate) fn as_element(&self) -> Option<&ElementData> {
        match self.data {
            NodeData::Element(ref data) => Some(data),
            _ => None
        }
    }

    pub(crate) fn as_text(&self) -> Option<&RefCell<StrTendril>> {
        match self.data {
            NodeData::Text { ref contents } => Some(contents),
            _ => None
        }
    }

    fn new(data: NodeData) -> Self {
        Node {
            parent: Cell::new(None),
            previous_sibling: Cell::new(None),
            next_sibling: Cell::new(None),
            first_child: Cell::new(None),
            last_child: Cell::new(None),
            data: data,
        }
    }

    fn detach(&self) {
        let parent = self.parent.take();
        let previous_sibling = self.previous_sibling.take();
        let next_sibling = self.next_sibling.take();

        if let Some(next_sibling) = next_sibling {
            next_sibling.previous_sibling.set(previous_sibling);
        } else if let Some(parent) = parent {
            parent.last_child.set(previous_sibling);
        }

        if let Some(previous_sibling) = previous_sibling {
            previous_sibling.next_sibling.set(next_sibling);
        } else if let Some(parent) = parent {
            parent.first_child.set(next_sibling);
        }
    }

    fn append(&'arena self, new_child: &'arena Self) {
        new_child.detach();
        new_child.parent.set(Some(self));
        if let Some(last_child) = self.last_child.take() {
            new_child.previous_sibling.set(Some(last_child));
            debug_assert!(last_child.next_sibling.get().is_none());
            last_child.next_sibling.set(Some(new_child));
        } else {
            debug_assert!(self.first_child.get().is_none());
            self.first_child.set(Some(new_child));
        }
        self.last_child.set(Some(new_child));
    }

    fn insert_before(&'arena self, new_sibling: &'arena Self) {
        new_sibling.detach();
        new_sibling.parent.set(self.parent.get());
        new_sibling.next_sibling.set(Some(self));
        if let Some(previous_sibling) = self.previous_sibling.take() {
            new_sibling.previous_sibling.set(Some(previous_sibling));
            debug_assert!(ptr::eq::<Node>(previous_sibling.next_sibling.get().unwrap(), self));
            previous_sibling.next_sibling.set(Some(new_sibling));
        } else if let Some(parent) = self.parent.get() {
            debug_assert!(ptr::eq::<Node>(parent.first_child.get().unwrap(), self));
            parent.first_child.set(Some(new_sibling));
        }
        self.previous_sibling.set(Some(new_sibling));
    }

    /// <https://dom.spec.whatwg.org/#concept-child-text-content>
    fn child_text_content(&self) -> StrTendril {
        let mut link = self.first_child.get();
        let mut text = None;
        while let Some(child) = link {
            if let NodeData::Text { ref contents } = child.data {
                let contents = contents.borrow();
                match text {
                    None => text = Some(contents.clone()),
                    Some(ref mut text) => text.push_tendril(&contents),
                }
            }
            link = child.next_sibling.get();
        }
        text.unwrap_or_else(StrTendril::new)
    }
}

impl<'arena> fmt::Debug for Node<'arena> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ptr: *const Node = self;
        f.debug_tuple("Node").field(&ptr).finish()
    }
}
