use cssparser::{CowRcStr, ParseError};
use selectors::parser::SelectorParseErrorKind;

pub(super) type PropertyParseError<'i> = ParseError<'i, PropertyParseErrorKind<'i>>;

pub(super) enum PropertyParseErrorKind<'i> {
    UnknownProperty(CowRcStr<'i>),
    UnknownUnit(CowRcStr<'i>),
}

pub(super) enum RuleParseErrorKind<'i> {
    Selector(SelectorParseErrorKind<'i>),
}

impl<'i> From<SelectorParseErrorKind<'i>> for RuleParseErrorKind<'i> {
    fn from(e: SelectorParseErrorKind<'i>) -> Self {
        RuleParseErrorKind::Selector(e)
    }
}
