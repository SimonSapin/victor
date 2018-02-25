use cssparser::{Parser, Token};
use primitives::{CssPx, Length as EuclidLength};
use style::errors::{PropertyParseError, PropertyParseErrorKind};

pub trait Parse: Sized {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>>;
}

pub trait ToComputedValue {
    type Computed;
    fn to_computed(&self) -> Self::Computed;
}

/// <https://drafts.csswg.org/css-values/#lengths>
#[derive(Clone)]
pub enum Length {
    Px(EuclidLength<CssPx>)
}

impl Parse for Length {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        match *parser.next()? {
            Token::Dimension { value, ref unit, .. } => match_ignore_ascii_case!(unit,
                "px" => return Ok(Length::Px(EuclidLength::new(value))),
                _ => {}
            ),
            _ => {}
        }
        Err(parser.new_custom_error(PropertyParseErrorKind::Other))
    }
}

impl ToComputedValue for Length {
    type Computed = EuclidLength<CssPx>;
    fn to_computed(&self) -> Self::Computed {
        match *self {
            Length::Px(px) => px,
        }
    }
}

pub struct FourSides<T> {
    pub top: T,
    pub left: T,
    pub bottom: T,
    pub right: T,
}

impl<T> Parse for FourSides<T> where T: Parse + Clone {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        let top = T::parse(parser)?;

        let left = if let Ok(left) = parser.try(T::parse) {
            left
        } else {
            return Ok(FourSides {
                top: top.clone(),
                left: top.clone(),
                bottom: top.clone(),
                right: top,
            })
        };

        let bottom = if let Ok(bottom) = parser.try(T::parse) {
            bottom
        } else {
            return Ok(FourSides {
                top: top.clone(),
                left: left.clone(),
                bottom: top,
                right: left,
            })
        };

        let right = if let Ok(right) = parser.try(T::parse) {
            right
        } else {
            return Ok(FourSides {
                top: top,
                left: left.clone(),
                bottom: bottom,
                right: left,
            })
        };

        Ok(FourSides { top, left, bottom, right })
    }
}
