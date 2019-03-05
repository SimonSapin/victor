use crate::style::errors::PropertyParseError;
use crate::style::properties::{ComputedValues, ComputedValuesForLateCascade};
use cssparser::Parser;

mod border;
mod color;
mod display;
mod fonts;
mod generic;
mod length;
mod writing_modes;

pub(super) use self::generic::*;
pub(crate) use self::{border::*, color::*, display::*, fonts::*, length::*, writing_modes::*};

pub(super) trait Parse: Sized {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>>;
}

pub(super) struct CascadeContext<'a> {
    pub inherited: &'a ComputedValues,
    pub this: ComputedValuesForLateCascade<'a>,
}

pub(super) struct EarlyCascadeContext<'a> {
    pub inherited: &'a ComputedValues,
}

pub(super) trait SpecifiedValue {
    type SpecifiedValue;
}

pub(super) trait FromSpecified: SpecifiedValue {
    fn from_specified(specified: &Self::SpecifiedValue, context: &CascadeContext) -> Self;
}

pub(super) trait EarlyFromSpecified: SpecifiedValue {
    fn early_from_specified(
        specified: &Self::SpecifiedValue,
        context: &EarlyCascadeContext,
    ) -> Self;
}

#[derive(Copy, Clone, Parse)]
pub(super) enum CssWideKeyword {
    Inherit,
    Initial,
    Unset,
}
