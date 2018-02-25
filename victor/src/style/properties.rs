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
        #[repr($DiscriminantType)]
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
        }

        impl PropertyDeclaration {
            fn id(&self) -> $DiscriminantType {
                // #[repr(u8)] guarantees that an enum’s representation starts with a u8 tag:
                // https://rust-lang.github.io/rfcs/2195-really-tagged-unions.html
                let ptr: *const PropertyDeclaration = self;
                let ptr = ptr as *const $DiscriminantType;
                unsafe {
                    *ptr
                }
            }

            pub fn cascade_into(&self, computed: &mut ComputedValues) {
                static CASCADE_FNS: &'static [fn(&PropertyDeclaration, &mut ComputedValues)] = &[
                    $(
                        |declaration, computed| {
                            // https://rust-lang.github.io/rfcs/2195-really-tagged-unions.html
                            #[repr(C)]
                            struct Repr {
                                tag: $DiscriminantType,
                                value: $ValueType,
                            }
                            let ptr: *const PropertyDeclaration = declaration;
                            let ptr = ptr as *const Repr;
                            let declaration = unsafe {
                                &*ptr
                            };
                            computed.$ident = declaration.value.to_computed()
                        }
                    )+
                ];
                CASCADE_FNS[self.id() as usize](self, computed)
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
