use super::length::{Length, PxLength};
use super::{Parse, ToComputedValue};
use crate::style::errors::{PropertyParseError, PropertyParseErrorKind};
use cssparser::{Color, Parser};
use euclid;
use std::marker::PhantomData;

/// https://drafts.csswg.org/css-backgrounds/#typedef-line-style
#[derive(Copy, Clone, Parse, ComputedAsSpecified)]
pub enum LineStyle {
    None,
    Solid,
}

#[derive(Parse)]
enum LineWidthKeyword {
    Thin,
    Medium,
    Thick,
}

#[derive(Copy, Clone)]
pub struct LineWidth(pub Length);

#[derive(Copy, Clone)]
pub struct LineWidthComputed(pub PxLength);

impl LineWidth {
    pub const MEDIUM: LineWidthComputed = LineWidthComputed(euclid::Length(3., PhantomData));
}

impl ToComputedValue for LineWidth {
    type Computed = LineWidthComputed;
    fn to_computed(&self) -> Self::Computed {
        LineWidthComputed(self.0.to_computed())
    }
}

impl Parse for LineWidth {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        parser
            .r#try(Length::parse)
            .or_else(|_| {
                Ok(Length::Px(PxLength::new(
                    match LineWidthKeyword::parse(parser)? {
                        LineWidthKeyword::Thin => 1.0,
                        LineWidthKeyword::Medium => 3.0,
                        LineWidthKeyword::Thick => 5.0,
                    },
                )))
            })
            .map(LineWidth)
    }
}

macro_rules! parse_one_or_more {
    ($type: ty { $( $field: ident, )+ }) => {
        impl Parse for BorderSide {
            fn parse<'i, 't>(parser: &mut Parser<'i, 't>)
                -> Result<Self, PropertyParseError<'i>>
            {
                let mut values = Self::default();
                let mut any = false;
                loop {
                    $(
                        if values.$field.is_none() {
                            if let Ok(value) = parser.r#try(Parse::parse) {
                                values.$field = Some(value);
                                any = true;
                                continue
                            }
                        }
                    )+
                    break
                }
                if any {
                    Ok(values)
                } else {
                    Err(parser.new_custom_error(PropertyParseErrorKind::Other))
                }
            }
        }
    };
}

parse_one_or_more!(BorderSide {
    style,
    color,
    width,
});

#[derive(Default)]
pub struct BorderSide {
    pub style: Option<LineStyle>,
    pub color: Option<Color>,
    pub width: Option<LineWidth>,
}
