use cssparser::Parser;
use style::errors::PropertyParseError;
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
