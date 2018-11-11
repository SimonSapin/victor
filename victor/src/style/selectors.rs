use crate::dom::{Document, Node, NodeId};
use crate::style::errors::RuleParseErrorKind;
use cssparser::ToCss;
use html5ever::{LocalName, Namespace, Prefix};
use selectors::attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint};
use selectors::context::{MatchingContext, MatchingMode, QuirksMode};
use selectors::matching::{matches_selector, ElementSelectorFlags};
use std::fmt;

pub type SelectorList = selectors::SelectorList<Impl>;
pub type Selector = selectors::parser::Selector<Impl>;

pub(crate) fn matches(selector: &Selector, document: &Document, element: NodeId) -> bool {
    matches_selector(
        selector,
        0,
        None,
        &NodeRef {
            document,
            node: element,
        },
        &mut MatchingContext::new(MatchingMode::Normal, None, None, QuirksMode::NoQuirks),
        &mut |_, _| {},
    )
}

#[derive(Clone, Debug)]
pub struct Impl;

pub struct Parser;

#[derive(Clone, PartialEq, Eq)]
pub enum PseudoElement {}

#[derive(Clone, PartialEq, Eq)]
pub enum PseudoClass {}

impl selectors::parser::NonTSPseudoClass for PseudoClass {
    type Impl = Impl;
    fn is_active_or_hover(&self) -> bool {
        match *self {}
    }
}

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
}

impl<'i> selectors::parser::Parser<'i> for Parser {
    type Impl = Impl;
    type Error = RuleParseErrorKind<'i>;
}

impl selectors::parser::PseudoElement for PseudoElement {
    type Impl = Impl;
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match *self {}
    }
}

impl ToCss for PseudoClass {
    fn to_css<W>(&self, _dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        match *self {}
    }
}

#[derive(Copy, Clone)]
struct NodeRef<'a> {
    document: &'a Document,
    node: NodeId,
}

impl<'a> std::fmt::Debug for NodeRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.node.fmt(f)
    }
}

impl<'a> NodeRef<'a> {
    fn node(self) -> &'a Node {
        &self.document[self.node]
    }
}

fn find_element<'a, F>(
    document: &'a Document,
    first: Option<NodeId>,
    next: F,
) -> Option<NodeRef<'a>>
where
    F: Fn(&Node) -> Option<NodeId>,
{
    let mut node = first?;
    loop {
        if document[node].as_element().is_some() {
            return Some(NodeRef { document, node })
        }
        node = next(&document[node])?
    }
}

impl<'a> selectors::Element for NodeRef<'a> {
    type Impl = Impl;

    fn opaque(&self) -> selectors::OpaqueElement {
        selectors::OpaqueElement::new::<Node>(self.node())
    }

    fn parent_element(&self) -> Option<Self> {
        let parent = self.node().parent?;
        self.document[parent].as_element()?;
        Some(NodeRef {
            document: self.document,
            node: parent,
        })
    }

    fn next_sibling_element(&self) -> Option<Self> {
        find_element(self.document, self.node().next_sibling, |node| {
            node.next_sibling
        })
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        find_element(self.document, self.node().previous_sibling, |node| {
            node.previous_sibling
        })
    }

    fn is_html_element_in_html_document(&self) -> bool {
        self.node().as_element().unwrap().name.ns == ns!(html) && self.node().in_html_document()
    }

    fn local_name(&self) -> &LocalName {
        &self.node().as_element().unwrap().name.local
    }

    fn namespace(&self) -> &Namespace {
        &self.node().as_element().unwrap().name.ns
    }

    fn is_html_slot_element(&self) -> bool {
        false
    }

    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }

    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&Namespace>,
        local_name: &LocalName,
        operation: &AttrSelectorOperation<&String>,
    ) -> bool {
        self.node().as_element().unwrap().attrs.iter().any(|attr| {
            attr.name.local == *local_name
                && match *ns {
                    NamespaceConstraint::Any => true,
                    NamespaceConstraint::Specific(ns) => attr.name.ns == *ns,
                }
                && operation.eval_str(&attr.value)
        })
    }

    fn match_non_ts_pseudo_class<F>(
        &self,
        pseudo_class: &PseudoClass,
        _context: &mut MatchingContext<Self::Impl>,
        _flags_setter: &mut F,
    ) -> bool
    where
        F: FnMut(&Self, ElementSelectorFlags),
    {
        match *pseudo_class {}
    }

    fn match_pseudo_element(
        &self,
        pseudo_element: &PseudoElement,
        _context: &mut MatchingContext<Self::Impl>,
    ) -> bool {
        match *pseudo_element {}
    }

    fn is_link(&self) -> bool {
        let element = self.node().as_element().unwrap();
        element.name.ns == ns!(html)
            && matches!(
                element.name.local,
                local_name!("a") | local_name!("area") | local_name!("link")
            )
            && element.get_attr(&local_name!("href")).is_some()
    }

    fn has_id(&self, id: &String, case_sensitivity: CaseSensitivity) -> bool {
        self.node()
            .as_element()
            .unwrap()
            .get_attr(&local_name!("id"))
            .map_or(false, |attr| {
                case_sensitivity.eq(id.as_bytes(), attr.as_bytes())
            })
    }

    fn has_class(&self, class: &String, case_sensitivity: CaseSensitivity) -> bool {
        self.node()
            .as_element()
            .unwrap()
            .get_attr(&local_name!("class"))
            .map_or(false, |attr| {
                case_sensitivity.eq(class.as_bytes(), attr.as_bytes())
            })
    }

    fn is_empty(&self) -> bool {
        match self.node().first_child {
            None => true,
            Some(mut node) => loop {
                if self.document[node].as_element().is_some() {
                    return false
                }
                if let Some(text) = self.document[node].as_text() {
                    if !text.is_empty() {
                        return false
                    }
                }
                match self.document[node].next_sibling {
                    None => return true,
                    Some(n) => node = n,
                }
            },
        }
    }

    fn is_root(&self) -> bool {
        self.parent_element().is_none()
    }
}
