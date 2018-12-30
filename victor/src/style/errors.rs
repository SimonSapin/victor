use cssparser::{CowRcStr, ParseError};
use selectors::parser::SelectorParseErrorKind;

pub type PropertyParseError<'i> = ParseError<'i, PropertyParseErrorKind<'i>>;

pub enum PropertyParseErrorKind<'i> {
    UnknownProperty(CowRcStr<'i>),
    UnknownUnit(CowRcStr<'i>),
}

pub enum RuleParseErrorKind<'i> {
    Selector(SelectorParseErrorKind<'i>),
}

impl<'i> From<SelectorParseErrorKind<'i>> for RuleParseErrorKind<'i> {
    fn from(e: SelectorParseErrorKind<'i>) -> Self {
        RuleParseErrorKind::Selector(e)
    }
}
