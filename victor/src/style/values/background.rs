use crate::style::errors::PropertyParseError;
use cssparser::{Color, Parser};

pub(in crate::style) struct Background {
    pub color: Option<Color>,
}

impl super::Parse for Background {
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        Ok(Background {
            color: Some(Color::parse(parser)?),
        })
    }
}
