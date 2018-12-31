pub(crate) use self::definitions::ComputedValues;
pub(super) use self::definitions::{property_data_by_name, LonghandDeclaration};
use self::definitions::{LonghandId, INITIAL_VALUES};
use crate::style::errors::PropertyParseError;
use crate::style::values::CssWideKeyword;
use std::rc::Rc;

#[macro_use]
mod macros;

mod definitions;

impl ComputedValues {
    pub(crate) fn initial() -> Rc<Self> {
        INITIAL_VALUES.with(|initial| initial.clone())
    }

    pub(crate) fn anonymous_inheriting_from(parent_style: &Self) -> Rc<Self> {
        INITIAL_VALUES.with(|initial| Rc::new(Self::new_inheriting_from(parent_style, initial)))
    }
}

type FnParseProperty = for<'i, 't> fn(
    &mut cssparser::Parser<'i, 't>,
    &mut Vec<LonghandDeclaration>,
) -> Result<(), PropertyParseError<'i>>;

pub struct PropertyData {
    pub(in crate::style) longhands: &'static [LonghandId],
    pub(in crate::style) parse: FnParseProperty,
}

trait ValueOrInitial<T> {
    fn into<F>(self, id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration;
}

impl<T> ValueOrInitial<T> for T {
    fn into<F>(self, _id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration,
    {
        constructor(self)
    }
}

impl<T> ValueOrInitial<T> for Option<T> {
    fn into<F>(self, id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration,
    {
        match self {
            Some(value) => constructor(value),
            None => LonghandDeclaration::CssWide(id, CssWideKeyword::Initial),
        }
    }
}
