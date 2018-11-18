use crate::style::errors::PropertyParseError;
use cssparser::Parser;

pub mod generic;
pub mod length;

pub trait Parse: Sized {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>>;
}

pub trait ToComputedValue {
    type Computed;
    fn to_computed(&self) -> Self::Computed;
}

/// https://drafts.csswg.org/css-display-3/#the-display-properties
#[derive(Copy, Clone, ComputedAsSpecified)]
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
