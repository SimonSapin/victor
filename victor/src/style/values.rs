use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser};

pub mod border;
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

/// https://drafts.csswg.org/css-display-3/#the-display-properties
#[derive(Copy, Clone, SpecifiedAsComputed)]
pub enum Display {
    None,
    Other {
        outside: DisplayOutside,
        inside: DisplayInside,
    },
}

#[derive(Copy, Clone)]
pub enum DisplayOutside {
    Inline,
    Block,
}

#[derive(Copy, Clone)]
pub enum DisplayInside {
    Flow,
}

impl Parse for Display {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        let ident = parser.expect_ident()?;
        match &**ident {
            "none" => Ok(Display::None),
            "block" => Ok(Display::Other {
                outside: DisplayOutside::Block,
                inside: DisplayInside::Flow,
            }),
            "inline" => Ok(Display::Other {
                outside: DisplayOutside::Inline,
                inside: DisplayInside::Flow,
            }),
            _ => {
                let token = cssparser::Token::Ident(ident.clone());
                Err(parser.new_unexpected_token_error(token))
            }
        }
    }
}
