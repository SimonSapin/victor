pub(crate) use self::definitions::ComputedValues;
use self::definitions::LonghandId;
pub(super) use self::definitions::{property_data_by_name, LonghandDeclaration};
pub(super) use self::definitions::{ComputedValuesForEarlyCascade, ComputedValuesForLateCascade};
use crate::geom::{flow_relative, physical};
use crate::style::errors::PropertyParseError;
use crate::style::values::{self, CssWideKeyword, Direction, WritingMode};
use crate::style::values::{CascadeContext, EarlyCascadeContext};
use cssparser::{Color, RGBA};
use std::sync::Arc;

#[macro_use]
mod macros;

mod definitions;

impl ComputedValues {
    pub(crate) fn anonymous_inheriting_from(parent_style: Option<&Self>) -> Arc<Self> {
        Self::new(parent_style, None)
    }

    pub(super) fn post_cascade_fixups(&mut self) {
        let b = Arc::make_mut(&mut self.border);
        b.border_top_width.fixup(b.border_top_style);
        b.border_left_width.fixup(b.border_left_style);
        b.border_bottom_width.fixup(b.border_bottom_style);
        b.border_right_width.fixup(b.border_right_style);
    }

    pub(crate) fn writing_mode(&self) -> (WritingMode, Direction) {
        // FIXME: For now, this is the only supported mode
        (WritingMode::HorizontalTb, Direction::Ltr)
    }

    pub(crate) fn box_offsets(&self) -> flow_relative::Sides<values::LengthOrPercentageOrAuto> {
        physical::Sides {
            top: self.box_.top,
            left: self.box_.left,
            bottom: self.box_.bottom,
            right: self.box_.right,
        }
        .to_flow_relative(self.writing_mode())
    }

    pub(crate) fn box_size(&self) -> flow_relative::Vec2<values::LengthOrPercentageOrAuto> {
        physical::Vec2 {
            x: self.box_.width,
            y: self.box_.height,
        }
        .size_to_flow_relative(self.writing_mode())
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

    pub(crate) fn to_rgba(&self, color: Color) -> RGBA {
        match color {
            Color::RGBA(rgba) => rgba,
            Color::CurrentColor => self.color.color,
        }
    }
}

pub(super) trait Phase {
    fn select(&self, p: PerPhase<bool>) -> bool;
    fn cascade(&mut self, declaration: &LonghandDeclaration);
}

impl Phase for EarlyCascadeContext<'_> {
    fn select(&self, p: PerPhase<bool>) -> bool {
        p.early
    }

    fn cascade(&mut self, declaration: &LonghandDeclaration) {
        declaration.if_early_cascade_into(self)
    }
}

impl Phase for CascadeContext<'_> {
    fn select(&self, p: PerPhase<bool>) -> bool {
        p.late
    }

    fn cascade(&mut self, declaration: &LonghandDeclaration) {
        declaration.if_late_cascade_into(self)
    }
}

#[derive(Default, Copy, Clone)]
pub(super) struct PerPhase<T> {
    pub early: T,
    pub late: T,
}

type FnParseProperty = for<'i, 't> fn(
    &mut cssparser::Parser<'i, 't>,
    &mut Vec<LonghandDeclaration>,
) -> Result<PerPhase<bool>, PropertyParseError<'i>>;

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
