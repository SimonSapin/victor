use cssparser::ToCss;
use html5ever::{LocalName, Namespace, Prefix};
use selectors;
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
