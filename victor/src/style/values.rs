use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser};

pub(crate) mod border;
pub(crate) mod display;
pub(crate) mod generic;
pub(crate) mod length;

pub(super) trait Parse: Sized {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>>;
}

pub(super) trait FromSpecified {
    type SpecifiedValue;
    fn from_specified(specified: &Self::SpecifiedValue) -> Self;
}

impl Parse for Color {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        Ok(Color::parse(parser)?)
    }
}

impl FromSpecified for Color {
    type SpecifiedValue = Self;
    fn from_specified(specified: &Self) -> Self {
        specified.clone()
    }
}

#[derive(Copy, Clone, Parse)]
pub(super) enum CssWideKeyword {
    Inherit,
    Initial,
    Unset,
}
