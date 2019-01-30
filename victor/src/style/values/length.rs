use crate::style::errors::{PropertyParseError, PropertyParseErrorKind};
use crate::style::values::{FromSpecified, Parse};
use cssparser::{Parser, Token};
use std::ops::{Add, Mul};

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub(crate) struct Length {
    pub px: f32,
}

/// <https://drafts.csswg.org/css-values/#percentages>
#[repr(transparent)]
#[derive(Copy, Clone, SpecifiedAsComputed)]
pub(crate) struct Percentage {
    unit_value: f32,
}

/// <https://drafts.csswg.org/css-values/#lengths>
#[derive(Clone, FromVariants)]
pub(in crate::style) enum SpecifiedLength {
    Absolute(Length),
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

impl Parse for SpecifiedLength {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        match parser.next()? {
            Token::Dimension { value, unit, .. } => match_ignore_ascii_case!(unit,
                "px" => Ok(SpecifiedLength::Absolute(Length { px: *value })),
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
            SpecifiedLength::Absolute(px) => *px,
        }
    }
}

impl Parse for Percentage {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        Ok(Percentage {
            unit_value: parser.expect_percentage()?,
        })
    }
}

impl Length {
    pub fn zero() -> Self {
        Length { px: 0. }
    }
}

impl Add for Length {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Length {
            px: self.px + other.px,
        }
    }
}

impl Mul<Percentage> for Length {
    type Output = Self;

    fn mul(self, other: Percentage) -> Self {
        Length {
            px: self.px * other.unit_value,
        }
    }
}

impl LengthOrPercentage {
    pub(crate) fn percentage_relative_to(&self, reference: Length) -> Length {
        match *self {
            LengthOrPercentage::Length(l) => l,
            LengthOrPercentage::Percentage(p) => reference * p,
        }
    }
}

impl LengthOrPercentageOrAuto {
    pub(crate) fn auto_is(&self, auto_value: Length) -> LengthOrPercentage {
        match *self {
            LengthOrPercentageOrAuto::Length(l) => LengthOrPercentage::Length(l),
            LengthOrPercentageOrAuto::Percentage(p) => LengthOrPercentage::Percentage(p),
            LengthOrPercentageOrAuto::Auto => LengthOrPercentage::Length(auto_value),
        }
    }
}
