use crate::style::errors::PropertyParseError;
use crate::style::values::Parse;
use cssparser::Parser;

pub struct FourSides<T> {
    pub top: T,
    pub left: T,
    pub bottom: T,
    pub right: T,
}

impl<T> Parse for FourSides<T>
where
    T: Parse + Clone,
{
    fn parse<'i, 't>(parser: &mut Parser<'i, 't>) -> Result<Self, PropertyParseError<'i>> {
        let top = T::parse(parser)?;

        let left = if let Ok(left) = parser.r#try(T::parse) {
            left
        } else {
            return Ok(FourSides {
                top: top.clone(),
                left: top.clone(),
                bottom: top.clone(),
                right: top,
            })
        };

        let bottom = if let Ok(bottom) = parser.r#try(T::parse) {
            bottom
        } else {
            return Ok(FourSides {
                top: top.clone(),
                left: left.clone(),
                bottom: top,
                right: left,
            })
        };

        let right = if let Ok(right) = parser.r#try(T::parse) {
            right
        } else {
            return Ok(FourSides {
                top: top,
                left: left.clone(),
                bottom: bottom,
                right: left,
            })
        };

        Ok(FourSides {
            top,
            left,
            bottom,
            right,
        })
    }
}
