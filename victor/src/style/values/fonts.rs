use super::{EarlyCascadeContext, EarlyFromSpecified, Length, SpecifiedLength, SpecifiedValue};

#[derive(Clone)]
pub(crate) struct FontSize(Length);

impl From<Length> for FontSize {
    fn from(l: Length) -> Self {
        FontSize(l)
    }
}

impl SpecifiedValue for FontSize {
    type SpecifiedValue = SpecifiedLength;
}

impl EarlyFromSpecified for FontSize {
    fn early_from_specified(s: &SpecifiedLength, _: &EarlyCascadeContext) -> Self {
        match s {
            SpecifiedLength::Absolute(px) => FontSize(*px),
        }
    }
}
