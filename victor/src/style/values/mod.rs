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
