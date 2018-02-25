use cssparser::Parser;
use primitives::{Length as EuclidLength};
use style::errors::PropertyParseError;
use style::values::{Parse, ToComputedValue, Length};

type FnParseProperty =
    for<'i, 't>
    fn(&mut Parser<'i, 't>)
    -> Result<PropertyDeclaration, PropertyParseError<'i>>;

macro_rules! properties {
    (
        type Discriminant = $DiscriminantType: ident;
        $(
            $ident: ident {
                name: $name: expr,
                specified: $ValueType: ty,
                initial: $initial_value: expr,
            }
        )+
    ) => {
        #[allow(non_camel_case_types)]
        pub enum PropertyDeclaration {
            $(
                $ident($ValueType),
            )+
        }

        pub struct ComputedValues {
            $(
                $ident: <$ValueType as ToComputedValue>::Computed,
            )+
        }

        impl ComputedValues {
            pub fn initial() -> Self {
                ComputedValues {
                    $(
                        $ident: $initial_value,
                    )+
                }
            }

            pub fn computed_declaration(&mut self, declaration: &PropertyDeclaration) {
                match *declaration {
                    $(
                        PropertyDeclaration::$ident(ref value) => {
                            self.$ident = value.to_computed()
                        }
                    )+
                }
            }
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
                            <$ValueType as Parse>::parse(parser).map(PropertyDeclaration::$ident)
                        };
                        PARSE
                    },
                )+
            }
        }
    }
}

properties! {
    type Discriminant = u8;

    font_size {
        name: "font-size",
        specified: Length,
        initial: EuclidLength::new(16.),
    }
}
