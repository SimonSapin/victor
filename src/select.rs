use selectors;
use selectors::parser::{self, SelectorImpl, Selector, AttrSelector, NamespaceConstraint};
use string_cache::{Atom, Namespace};
use xml::{Element, Ref, NodeData};

pub struct SelectorList(Vec<Selector<Impl>>);

impl SelectorList {
    pub fn parse(s: &str) -> Result<Self, ()> {
        parser::parse_author_origin_selector_list_from_str(s).map(SelectorList)
    }

    pub fn matches(&self, element: Element) -> bool {
        selectors::matching::matches(&self.0, &element, None)
    }

    /// Return the first element that matches, if any,
    /// among this node and its descendants in tree order.
    pub fn query<'arena>(&self, node: Ref<'arena>) -> Option<Element<'arena>> {
        if let Some(element) = node.as_element() {
            if self.matches(element) {
                return Some(element)
            }
        }
        let mut link = node.first_child();
        while let Some(child_node) = link {
            if let Some(matching_element) = self.query(child_node) {
                return Some(matching_element)
            }
            link = child_node.next_sibling()
        }
        None
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
    ($($method: ident => $link: ident,)+) => {
        $(
            fn $method(&self) -> Option<Self> {
                let mut link = self.node.$link();
                while let Some(node) = link {
                    if let Some(element) = node.as_element() {
                        return Some(element)
                    }
                    link = node.$link()
                }
                None
            }
        )+
    }
}

impl<'arena> selectors::Element for Element<'arena> {
    type Impl = Impl;

    traversal_methods! {
        parent_element => parent,
        first_child_element => first_child,
        last_child_element => last_child,
        prev_sibling_element => previous_sibling,
        next_sibling_element => next_sibling,
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
        &self.data.name.local
    }

    fn get_namespace(&self) -> &Namespace {
        &self.data.name.ns
    }

    fn get_id(&self) -> Option<Atom> {
        self.data.id.clone()
    }

    fn has_class(&self, name: &Atom) -> bool {
        self.data.classes.contains(name)
    }

    fn each_class<F>(&self, mut callback: F) where F: FnMut(&Atom) {
        for class in &self.data.classes {
            callback(class)
        }
    }

    fn match_attr<F>(&self, selector: &AttrSelector, test: F) -> bool where F: Fn(&str) -> bool {
        self.data.attributes.iter().any(|&(ref name, ref value)| {
            name.local == selector.name &&
            match selector.namespace {
                NamespaceConstraint::Specific(ref selector_ns) => name.ns == *selector_ns,
                NamespaceConstraint::Any => true
            } &&
            test(value)
        })
    }

    fn is_empty(&self) -> bool {
        let mut link = self.node.first_child();
        while let Some(node) = link {
            match node.data {
                NodeData::Element(_) => return false,
                NodeData::Text(ref text) if !text.is_empty() => return false,
                _ => {}
            }
            link = node.next_sibling()
        }
        true
    }
}
