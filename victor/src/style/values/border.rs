use super::length::{Length, SpecifiedLength};
use super::Parse;
use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser};
use euclid;
use std::marker::PhantomData;

/// https://drafts.csswg.org/css-backgrounds/#typedef-line-style
#[derive(Copy, Clone, Parse, SpecifiedAsComputed)]
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

#[derive(Clone)]
pub struct SpecifiedLineWidth(pub SpecifiedLength);

#[derive(Copy, Clone, FromSpecified)]
pub struct LineWidth(pub Length);

impl LineWidth {
    pub const MEDIUM: Self = LineWidth(euclid::Length(3., PhantomData));
}

impl Parse for SpecifiedLineWidth {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        parser
            .r#try(SpecifiedLength::parse)
            .or_else(|_| {
                Ok(SpecifiedLength::Px(Length::new(
                    match LineWidthKeyword::parse(parser)? {
                        LineWidthKeyword::Thin => 1.0,
                        LineWidthKeyword::Medium => 3.0,
                        LineWidthKeyword::Thick => 5.0,
                    },
                )))
            })
            .map(SpecifiedLineWidth)
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
                    Err(parser.new_error_for_next_token())
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
    pub width: Option<SpecifiedLineWidth>,
}
