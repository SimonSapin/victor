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
