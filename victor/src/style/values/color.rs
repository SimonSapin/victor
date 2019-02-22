use super::{FromSpecified, Parse};
use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser, RGBA};

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

impl Parse for RGBA {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        match Color::parse(parser)? {
            Color::RGBA(rgba) => Ok(rgba),
            Color::CurrentColor => Err(parser.new_error_for_next_token()),
        }
    }
}

impl FromSpecified for RGBA {
    type SpecifiedValue = Self;
    fn from_specified(specified: &Self) -> Self {
        specified.clone()
    }
}

pub(crate) const BLACK: RGBA = RGBA {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 255,
};
