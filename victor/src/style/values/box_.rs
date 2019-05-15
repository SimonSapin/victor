use crate::style::errors::PropertyParseError;
use crate::style::properties::ComputedValues;
use cssparser::Parser;
use std::sync::Arc;

/// https://drafts.csswg.org/css-display-3/#the-display-properties
#[derive(Copy, Clone, Eq, PartialEq, SpecifiedAsComputed)]
pub(crate) enum Display {
    None,
    Contents,
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
    pub const INITIAL: Self = Display::Other {
        outside: DisplayOutside::Inline,
        inside: DisplayInside::Flow,
    };

    /// https://drafts.csswg.org/css-display-3/#blockify
    pub fn blockify(&self) -> Self {
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

    /// https://drafts.csswg.org/css2/visuren.html#dis-pos-flo
    pub fn fixup(style: &mut ComputedValues) {
        style.specified_display = style.box_.display;
        if style.box_.position.is_absolutely_positioned() || style.box_.float.is_floating() {
            let display = style.box_.display.blockify();
            if display != style.box_.display {
                Arc::make_mut(&mut style.box_).display = display
            }
        }
    }
}

impl super::Parse for Display {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        let ident = parser.expect_ident()?;
        match &**ident {
            "none" => Ok(Display::None),
            "contents" => Ok(Display::Contents),
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

/// https://drafts.csswg.org/css2/visuren.html#propdef-float
#[derive(Copy, Clone, Eq, Parse, PartialEq, SpecifiedAsComputed)]
pub(crate) enum Float {
    None,
    Left,
    Right,
}

impl Float {
    pub fn is_floating(self) -> bool {
        match self {
            Float::None => false,
            Float::Left | Float::Right => true,
        }
    }
}

/// https://drafts.csswg.org/css-position-3/#position-property
#[derive(Copy, Clone, Eq, Parse, PartialEq, SpecifiedAsComputed)]
pub(crate) enum Position {
    Static,
    Relative,
    Absolute,
}

impl Position {
    pub fn is_relatively_positioned(self) -> bool {
        self == Position::Relative
    }

    pub fn is_absolutely_positioned(self) -> bool {
        self == Position::Absolute
    }
}
