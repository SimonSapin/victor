use selectors;
use selectors::parser::{self, SelectorImpl, Selector, AttrSelector, NamespaceConstraint};
use string_cache::{Atom, Namespace};
use xml::{Node, Cursor};


pub struct SelectorList(Vec<Selector<Impl>>);

impl SelectorList {
    pub fn parse(s: &str) -> Result<Self, ()> {
        parser::parse_author_origin_selector_list_from_str(s).map(SelectorList)
    }

    pub fn matches(&self, cursor: &Cursor) -> bool {
        selectors::matching::matches(&self.0, cursor, None)
    }

    /// Return the next element that matches, if any,
    /// among this node and its successors in tree order.
    pub fn query_next(&self, cursor: &mut Cursor) -> bool {
        loop {
            if self.matches(&cursor) {
                return true
            }
            if !cursor.next_in_tree_order() {
                return false
            }
        }
    }
}

pub struct Impl;
#[derive(Clone, Debug, PartialEq)] pub enum NonTSPseudoClass {}
#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub enum PseudoElement {}

impl SelectorImpl for Impl {
    type NonTSPseudoClass = NonTSPseudoClass;
    type PseudoElement = PseudoElement;
}

macro_rules! traversal_methods {
    ($($method: ident,)+) => {
        $(
            fn $method(&self) -> Option<Self> {
                let mut new_cursor = self.clone();
                if Cursor::$method(&mut new_cursor) {
                    Some(new_cursor)
                } else {
                    None
                }
            }
        )+
    }
}

impl<'document> selectors::Element for Cursor<'document> {
    type Impl = Impl;

    traversal_methods! {
        parent_element,
        first_child_element,
        last_child_element,
        prev_sibling_element,
        next_sibling_element,
    }

    fn is_html_element_in_html_document(&self) -> bool {
        false
    }

    fn match_non_ts_pseudo_class(&self, pc: NonTSPseudoClass) -> bool {
        match pc {}
    }

    fn is_root(&self) -> bool {
        self.parent_element().is_none()
    }

    fn get_local_name(&self) -> &Atom {
        &self.element().name.local
    }

    fn get_namespace(&self) -> &Namespace {
        &self.element().name.ns
    }

    fn get_id(&self) -> Option<Atom> {
        self.element().id.clone()
    }

    fn has_class(&self, name: &Atom) -> bool {
        self.element().classes.contains(name)
    }

    fn each_class<F>(&self, mut callback: F) where F: FnMut(&Atom) {
        for class in &self.element().classes {
            callback(class)
        }
    }

    fn match_attr<F>(&self, selector: &AttrSelector, test: F) -> bool where F: Fn(&str) -> bool {
        self.element().attributes.iter().any(|&(ref name, ref value)| {
            name.local == selector.name &&
            match selector.namespace {
                NamespaceConstraint::Specific(ref selector_ns) => name.ns == *selector_ns,
                NamespaceConstraint::Any => true
            } &&
            test(value)
        })
    }

    fn is_empty(&self) -> bool {
        self.element().children.iter().all(|child| match *child {
            Node::Element(_) => false,
            Node::Text(ref text) => text.is_empty(),
            _ => true,
        })
    }
}

#[test]
fn test_selectors() {
    let source: &[u8] = b"<a>foo <b/></a>";
    let doc = Node::parse(source).unwrap();
    let mut cursor = doc.cursor();
    assert_eq!(cursor.element().name.local, atom!("a"));
    assert!(cursor.first_child_element());
    assert_eq!(cursor.element().name.local, atom!("b"));

}
