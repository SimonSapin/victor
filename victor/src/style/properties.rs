pub(crate) use self::definitions::ComputedValues;
pub(super) use self::definitions::{property_data_by_name, LonghandDeclaration};
use self::definitions::{LonghandId, INITIAL_VALUES};
use crate::geom::{flow_relative, physical};
use crate::style::errors::PropertyParseError;
use crate::style::values::{self, CssWideKeyword, Direction, WritingMode};
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

    pub(crate) fn writing_mode(&self) -> (WritingMode, Direction) {
        // FIXME: For now, this is the only supported mode
        (WritingMode::HorizontalTb, Direction::Ltr)
    }

    pub(crate) fn padding(&self) -> flow_relative::Sides<values::LengthOrPercentage> {
        physical::Sides {
            top: self.padding.padding_top,
            left: self.padding.padding_left,
            bottom: self.padding.padding_bottom,
            right: self.padding.padding_right,
        }
        .to_flow_relative(self.writing_mode())
    }

    pub(crate) fn border_width(&self) -> flow_relative::Sides<values::LengthOrPercentage> {
        physical::Sides {
            top: self.border.border_top_width.0,
            left: self.border.border_left_width.0,
            bottom: self.border.border_bottom_width.0,
            right: self.border.border_right_width.0,
        }
        .to_flow_relative(self.writing_mode())
    }

    pub(crate) fn margin(&self) -> flow_relative::Sides<values::LengthOrPercentageOrAuto> {
        physical::Sides {
            top: self.margin.margin_top,
            left: self.margin.margin_left,
            bottom: self.margin.margin_bottom,
            right: self.margin.margin_right,
        }
        .to_flow_relative(self.writing_mode())
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
