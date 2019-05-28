use crate::style::errors::PropertyParseError;
use crate::style::properties::ComputedValues;
use cssparser::Parser;
use std::sync::Arc;

/// https://drafts.csswg.org/css-display-3/#the-display-properties
#[derive(Copy, Clone, Eq, PartialEq, SpecifiedAsComputed)]
pub(crate) enum Display {
    None,
    Contents,
    GeneratingBox(DisplayGeneratingBox),
}

#[allow(dead_code)]
fn _static_assert_size_of() {
    let _ = std::mem::transmute::<Display, [u8; 2]>;
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum DisplayGeneratingBox {
    OutsideInside {
        outside: DisplayOutside,
        inside: DisplayInside,
        // list_item: bool,
    },
    // Layout-internal display types go here:
    // https://drafts.csswg.org/css-display-3/#layout-specific-display
}

/// https://drafts.csswg.org/css-display-3/#outer-role
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum DisplayOutside {
    Inline,
    Block,
}

/// https://drafts.csswg.org/css-display-3/#inner-model
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) enum DisplayInside {
    Flow,
    FlowRoot,
}

impl Display {
    pub const INITIAL: Self = Display::GeneratingBox(DisplayGeneratingBox::OutsideInside {
        outside: DisplayOutside::Inline,
        inside: DisplayInside::Flow,
    });

    /// https://drafts.csswg.org/css-display-3/#blockify
    pub fn blockify(&self) -> Self {
        match *self {
            Display::GeneratingBox(value) => Display::GeneratingBox(match value {
                DisplayGeneratingBox::OutsideInside { outside: _, inside } => {
                    DisplayGeneratingBox::OutsideInside {
                        outside: DisplayOutside::Block,
                        inside,
                    }
                }
                // other => other,
            }),
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
            "block" => Ok(Display::GeneratingBox(
                DisplayGeneratingBox::OutsideInside {
                    outside: DisplayOutside::Block,
                    inside: DisplayInside::Flow,
                },
            )),
            "flow-root" => Ok(Display::GeneratingBox(
                DisplayGeneratingBox::OutsideInside {
                    outside: DisplayOutside::Block,
                    inside: DisplayInside::FlowRoot,
                },
            )),
            "inline" => Ok(Display::GeneratingBox(
                DisplayGeneratingBox::OutsideInside {
                    outside: DisplayOutside::Inline,
                    inside: DisplayInside::Flow,
                },
            )),
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
