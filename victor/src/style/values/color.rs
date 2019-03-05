use super::{FromSpecified, Parse};
use crate::style::errors::PropertyParseError;
use crate::style::values::CascadeContext;
use cssparser::{Color, Parser, RGBA};

impl Parse for Color {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        Ok(Color::parse(parser)?)
    }
}

impl FromSpecified for Color {
    type SpecifiedValue = Self;
    fn from_specified(specified: &Self, _: &CascadeContext) -> Self {
        specified.clone()
    }
}

// Only used for the 'color' property
impl FromSpecified for RGBA {
    type SpecifiedValue = Color;
    fn from_specified(specified: &Color, context: &CascadeContext) -> Self {
        match specified {
            Color::RGBA(rgba) => *rgba,
            // https://drafts.csswg.org/css-color/#resolve-color-values
            // “If `currentcolor` is the specified value of the 'color' property,
            //  it is treated as if the specified value was `inherit`.”
            Color::CurrentColor => context.inherited.color.color,
        }
    }
}

pub(crate) const BLACK: RGBA = RGBA {
    red: 0,
    green: 0,
    blue: 0,
    alpha: 255,
};
