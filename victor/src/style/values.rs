use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser};

mod border;
mod display;
mod generic;
mod length;

pub(super) use self::generic::*;
pub(crate) use self::{border::*, display::*, length::*};

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
