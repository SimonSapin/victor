use cssparser::ToCss;
use dom::{Node, NodeRef, Link};
use html5ever::{LocalName, Namespace, Prefix};
use selectors;
use selectors::attr::{NamespaceConstraint, CaseSensitivity, AttrSelectorOperation};
use selectors::context::{MatchingContext, VisitedHandlingMode};
use selectors::matching::ElementSelectorFlags;
use std::fmt;

#[derive(Clone)]
pub struct Impl;

pub struct Parser;

#[derive(Clone, PartialEq, Eq)]
pub enum PseudoElement {}

#[derive(Clone, PartialEq, Eq)]
pub enum PseudoClass {}

pub type ParseError<'i> = selectors::parser::SelectorParseErrorKind<'i>;

impl selectors::parser::SelectorImpl for Impl {
    type ExtraMatchingData = ();
    type AttrValue = String;
    type Identifier = String;
    type ClassName = String;
    type LocalName = LocalName;
    type NamespaceUrl = Namespace;
    type NamespacePrefix = Prefix;
    type BorrowedNamespaceUrl = Namespace;
    type BorrowedLocalName = LocalName;
    type NonTSPseudoClass = PseudoClass;
    type PseudoElement = PseudoElement;

    fn is_active_or_hover(pseudo_class: &Self::NonTSPseudoClass) -> bool {
        match *pseudo_class {}
    }
}

impl<'i> selectors::parser::Parser<'i> for Parser {
    type Impl = Impl;
    type Error = ParseError<'i>;
}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = Impl;
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result where W: fmt::Write {
        match *self {}
    }
}

impl ToCss for PseudoClass {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result where W: fmt::Write {
        match *self {}
    }
}

fn find_element<'arena, F>(first: &'arena Link<'arena>, next: F) -> Option<NodeRef<'arena>>
    where F: Fn(NodeRef<'arena>) -> &'arena Link<'arena>
{
    let mut node = first.get()?;
    loop {
        if node.as_element().is_some() {
            return Some(node)
        }
        node = next(node).get()?
    }
}

impl<'arena> selectors::Element for NodeRef<'arena> {
    type Impl = Impl;

    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new::<Node>(*self)
    }

    fn parent_element(&self) -> Option<Self> {
        let parent = self.parent.get()?;
        parent.as_element()?;
        Some(parent)
    }

    fn first_child_element(&self) -> Option<Self> {
        find_element(&self.first_child, |node| &node.next_sibling)
    }

    fn last_child_element(&self) -> Option<Self> {
        find_element(&self.last_child, |node| &node.previous_sibling)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        find_element(&self.next_sibling, |node| &node.next_sibling)
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        find_element(&self.previous_sibling, |node| &node.previous_sibling)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.as_element().unwrap().name.ns == ns!(html) && self.in_html_document()
    }

    fn get_local_name(&self) -> &LocalName {
        &self.as_element().unwrap().name.local
    }

    fn get_namespace(&self) -> &Namespace {
        &self.as_element().unwrap().name.ns
    }

    fn attr_matches(&self, ns: &NamespaceConstraint<&Namespace>, local_name: &LocalName,
                    operation: &AttrSelectorOperation<&String>) -> bool {
        self.as_element().unwrap().attrs.borrow().iter().any(|attr| {
            attr.name.local == *local_name &&
            match *ns {
                NamespaceConstraint::Any => true,
                NamespaceConstraint::Specific(ns) => attr.name.ns == *ns
            } &&
            operation.eval_str(&attr.value)
        })
    }

    fn match_non_ts_pseudo_class<F>(
        &self,
        pseudo_class: &PseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
        _visited_handling: VisitedHandlingMode,
        _flags_setter: &mut F
    ) -> bool
    where
        F: FnMut(&Self, ElementSelectorFlags)
    {
        match *pseudo_class {}
    }

    fn match_pseudo_element(&self, pseudo_element: &PseudoElement,
                            _context: &mut MatchingContext<Self::Impl>) -> bool {
        match *pseudo_element {}
    }

    fn is_link(&self) -> bool {
        let element = self.as_element().unwrap();
        element.name.ns == ns!(html) &&
        matches!(element.name.local, local_name!("a") | local_name!("area") | local_name!("link")) &&
        element.get_attr(&local_name!("href")).is_some()
    }

    fn has_id(&self, id: &String, case_sensitivity: CaseSensitivity) -> bool {
        self.as_element().unwrap().get_attr(&local_name!("id")).map_or(false, |attr| {
            case_sensitivity.eq(id.as_bytes(), attr.as_bytes())
        })
    }

    fn has_class(&self, class: &String, case_sensitivity: CaseSensitivity) -> bool {
        self.as_element().unwrap().get_attr(&local_name!("class")).map_or(false, |attr| {
            case_sensitivity.eq(class.as_bytes(), attr.as_bytes())
        })
    }

    fn is_empty(&self) -> bool {
        match self.first_child.get() {
            None => true,
            Some(mut node) => {
                loop {
                    if node.as_element().is_some() {
                        return false
                    }
                    if let Some(text) = node.as_text() {
                        if !text.borrow().is_empty() {
                            return false
                        }
                    }
                    match node.next_sibling.get() {
                        None => return true,
                        Some(n) => node = n
                    }
                }
            }
        }
    }

    fn is_root(&self) -> bool {
        self.parent_element().is_none()
    }
}
