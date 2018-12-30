use crate::primitives::{CssPx, Length as EuclidLength};
use crate::style::errors::{PropertyParseError, PropertyParseErrorKind};
use crate::style::values::{FromSpecified, Parse};
use cssparser::{Parser, Token};

pub type Length = EuclidLength<CssPx>;

/// <https://drafts.csswg.org/css-values/#lengths>
#[derive(Clone)]
pub enum SpecifiedLength {
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
