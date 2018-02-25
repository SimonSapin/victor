use cssparser::Parser;
use style::errors::PropertyParseError;
use style::values::{Parse, Length};

type FnParseProperty =
    for<'i, 't>
    fn(&mut Parser<'i, 't>)
    -> Result<PropertyDeclaration, PropertyParseError<'i>>;

macro_rules! properties {
    (
        $( $Variant: ident($ValueType: ty) = $name: expr; )+
    ) => {
        pub enum PropertyDeclaration {
            $(
                $Variant($ValueType),
            )+
        }

        ascii_case_insensitive_phf_map! {
            declaration_parsing_function_by_name -> FnParseProperty = {
                $(
                    $name => {
                        // Using a constant works around a spurious borrow-checking error
                        // that I did not bother filing because it is fixed
                        // by MIR-based borrow-checking, so it’ll go away soon enough.
                        // FIXME: remove the indirection when NLL ships.
                        const PARSE: FnParseProperty = |parser| {
                            <$ValueType as Parse>::parse(parser).map(PropertyDeclaration::$Variant)
                        };
                        PARSE
                    },
                )+
            }
        }
    }
}

properties! {
    FontSize(Length) = "font-size";
}
