use crate::primitives::{CssPx, Length as EuclidLength};
use crate::style::errors::{PropertyParseError, PropertyParseErrorKind};
use crate::style::values::{FromSpecified, Parse};
use cssparser::{Parser, Token};

pub(crate) type Length = EuclidLength<CssPx>;

/// <https://drafts.csswg.org/css-values/#lengths>
#[derive(Clone, FromVariants)]
pub(in crate::style) enum SpecifiedLength {
    Px(Length),
}

impl Parse for SpecifiedLength {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        match parser.next()? {
            Token::Dimension { value, unit, .. } => match_ignore_ascii_case!(unit,
                "px" => Ok(SpecifiedLength::Px(Length::new(*value))),
                _ => {
                    let u = unit.clone();
                    Err(parser.new_custom_error(PropertyParseErrorKind::UnknownUnit(u)))
                }
            ),
            token => {
                let t = token.clone();
                Err(parser.new_unexpected_token_error(t))
            }
        }
    }
}

impl FromSpecified for Length {
    type SpecifiedValue = SpecifiedLength;
    fn from_specified(s: &SpecifiedLength) -> Self {
        match s {
            SpecifiedLength::Px(px) => *px,
        }
    }
}

/// https://drafts.csswg.org/css-values/#percentages
#[derive(Copy, Clone, SpecifiedAsComputed)]
pub(crate) struct Percentage(f32);

impl Parse for Percentage {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        Ok(Percentage(parser.expect_percentage()?))
    }
}

#[derive(Clone, Parse, FromVariants)]
pub(in crate::style) enum SpecifiedLengthOrPercentage {
    Length(SpecifiedLength),
    Percentage(Percentage),
}

#[derive(Copy, Clone, FromSpecified, FromVariants)]
pub(crate) enum LengthOrPercentage {
    Length(Length),
    Percentage(Percentage),
}

#[derive(Clone, Parse, FromVariants)]
pub(in crate::style) enum SpecifiedLengthOrPercentageOrAuto {
    Length(SpecifiedLength),
    Percentage(Percentage),
    Auto,
}

#[derive(Copy, Clone, FromSpecified, FromVariants)]
pub(crate) enum LengthOrPercentageOrAuto {
    Length(Length),
    Percentage(Percentage),
    Auto,
}
