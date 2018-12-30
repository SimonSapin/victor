use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser};

pub mod border;
pub mod display;
pub mod generic;
pub mod length;

pub trait Parse: Sized {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>>;
}

pub trait FromSpecified {
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
pub enum CssWideKeyword {
    Inherit,
    Initial,
    Unset,
}
