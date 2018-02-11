use cssparser::{Parser, ParseError, CowRcStr, AtRuleParser, DeclarationParser};
use style::errors::PropertyParseErrorKind;
use style::values::Length;

pub enum PropertyDeclaration {
    FontSize(Length)
}

pub struct PropertyDeclarationParser;

impl<'i> DeclarationParser<'i> for PropertyDeclarationParser {
    type Declaration = PropertyDeclaration;
    type Error = PropertyParseErrorKind<'i>;

    fn parse_value<'t>(&mut self, name: CowRcStr<'i>, parser: &mut Parser<'i, 't>)
                       -> Result<Self::Declaration, ParseError<'i, Self::Error>>
    {
        match_ignore_ascii_case! { &name,
            "font-size" => return Ok(PropertyDeclaration::FontSize(Length::parse(parser)?)),
            _ => {}
        }
        Err(parser.new_custom_error(PropertyParseErrorKind::UnknownProperty(name)))
    }
}

impl<'i> AtRuleParser<'i> for PropertyDeclarationParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = PropertyDeclaration;
    type Error = PropertyParseErrorKind<'i>;
}
