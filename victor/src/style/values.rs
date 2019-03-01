use crate::style::errors::PropertyParseError;
use crate::style::properties::CascadeContext;
use cssparser::Parser;

mod border;
mod color;
mod display;
mod generic;
mod length;
mod writing_modes;

pub(super) use self::generic::*;
pub(crate) use self::{border::*, color::*, display::*, length::*, writing_modes::*};

pub(super) trait Parse: Sized {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>>;
}

pub(super) trait FromSpecified {
    type SpecifiedValue;
    fn from_specified(specified: &Self::SpecifiedValue, context: &CascadeContext) -> Self;
}
#[derive(Copy, Clone, Parse)]
pub(super) enum CssWideKeyword {
    Inherit,
    Initial,
    Unset,
}
