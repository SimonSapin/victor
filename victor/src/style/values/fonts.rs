use super::{EarlyCascadeContext, EarlyFromSpecified, Length, SpecifiedLength, SpecifiedValue};

#[derive(Copy, Clone)]
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

impl std::ops::Mul<crate::primitives::Length<crate::fonts::Em>> for FontSize {
    type Output = Length;
    fn mul(self, other: crate::primitives::Length<crate::fonts::Em>) -> Length {
        self.0 * other.0
    }
}
