use super::{EarlyCascadeContext, EarlyFromSpecified, Length, SpecifiedLength, SpecifiedValue};

#[derive(Clone)]
pub(crate) struct FontSize(pub Length);

impl From<Length> for FontSize {
    fn from(l: Length) -> Self {
        FontSize(l)
    }
}

impl SpecifiedValue for FontSize {
    type SpecifiedValue = SpecifiedLength;
}

impl EarlyFromSpecified for FontSize {
    fn early_from_specified(s: &SpecifiedLength, context: &EarlyCascadeContext) -> Self {
        FontSize(match s {
            SpecifiedLength::Absolute(px) => *px,
            SpecifiedLength::Em(value) => context.inherited.font.font_size.0 * *value,
        })
    }
}
