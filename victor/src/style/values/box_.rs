use crate::style::errors::PropertyParseError;
use crate::style::values::{CascadeContext, FromSpecified, SpecifiedValue};
use cssparser::Parser;

/// https://drafts.csswg.org/css-display-3/#the-display-properties
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum Display {
    None,
    Other {
        outside: DisplayOutside,
        inside: DisplayInside,
    },
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum DisplayOutside {
    Inline,
    Block,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum DisplayInside {
    Flow,
}

impl Display {
    /// https://drafts.csswg.org/css-display-3/#blockify
    fn blockify(&self) -> Self {
        match *self {
            Display::Other {
                outside: DisplayOutside::Inline,
                inside,
            } => Display::Other {
                outside: DisplayOutside::Block,
                inside,
            },
            other => other,
        }
    }
}

impl SpecifiedValue for Display {
    type SpecifiedValue = Display;
}

impl FromSpecified for Display {
    /// https://drafts.csswg.org/css2/visuren.html#dis-pos-flo
    fn from_specified(specified: &Display, context: &CascadeContext) -> Self {
        if !context.this.position().is_absolutely_positioned() {
            *specified
        } else {
            specified.blockify()
        }
    }
}

impl super::Parse for Display {
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

/// https://drafts.csswg.org/css-position-3/#position-property
#[derive(Copy, Clone, Eq, Parse, PartialEq, SpecifiedAsComputed)]
pub(crate) enum Position {
    Static,
    Absolute,
}

impl Position {
    pub fn is_absolutely_positioned(&self) -> bool {
        *self == Position::Absolute
    }
}
