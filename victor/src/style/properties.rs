use cssparser::{Parser, ParseError, CowRcStr, AtRuleParser, DeclarationParser};
use style::errors::{PropertyParseError, PropertyParseErrorKind};
use style::values::Length;

pub enum PropertyDeclaration {
    FontSize(Length)
}

type FnParseProperty =
    for<'i, 't>
    fn(&mut Parser<'i, 't>)
    -> Result<PropertyDeclaration, PropertyParseError<'i>>;

ascii_case_insensitive_phf_map! {
    declaration_parsing_function_by_name -> FnParseProperty = {
        "font-size" => {
            // Using a constant works around a spurious borrow-checking error
            // that I did not bother filing because it is fixed by MIR-based borrow-checking,
            // so it’ll go away soon enough.
            // FIXME: remove the indirection when NLL ships.
            const PARSE: FnParseProperty = |parser| Length::parse(parser).map(PropertyDeclaration::FontSize);
            PARSE
        },
    }
}

pub struct PropertyDeclarationParser;

impl<'i> DeclarationParser<'i> for PropertyDeclarationParser {
    type Declaration = PropertyDeclaration;
    type Error = PropertyParseErrorKind<'i>;

    fn parse_value<'t>(&mut self, name: CowRcStr<'i>, parser: &mut Parser<'i, 't>)
                       -> Result<Self::Declaration, ParseError<'i, Self::Error>>
    {
        if let Some(parse) = declaration_parsing_function_by_name(&name) {
            parse(parser)
        } else {
            Err(parser.new_custom_error(PropertyParseErrorKind::UnknownProperty(name)))
        }
    }
}

impl<'i> AtRuleParser<'i> for PropertyDeclarationParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = PropertyDeclaration;
    type Error = PropertyParseErrorKind<'i>;
}
