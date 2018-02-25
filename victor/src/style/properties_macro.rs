macro_rules! properties {
    (
        type Discriminant = $DiscriminantType: ident;
        $(
            $inherited: ident struct $struct_name: ident {
                $(
                    $ident: ident {
                        name: $name: expr,
                        specified: $ValueType: ty,
                        initial: $initial_value: expr,
                    }
                )+
            }
        )+
    ) => {
        #[repr($DiscriminantType)]
        #[allow(non_camel_case_types)]
        pub enum PropertyDeclaration {
            $($(
                $ident($ValueType),
            )+)+
        }

        pub struct ComputedValues {
            $(
                pub $struct_name: ::std::rc::Rc<style_structs::$struct_name>,
            )+
        }

        pub mod style_structs {
            use super::*;
            $(
                #[allow(non_camel_case_types)]
                #[derive(Clone)]
                pub struct $struct_name {
                    $(
                        pub $ident: <$ValueType as ::style::values::ToComputedValue>::Computed,
                    )+
                }

                impl $struct_name {
                    pub fn initial() -> Self {
                        $struct_name {
                            $(
                                $ident: $initial_value,
                            )+
                        }
                    }
                }
            )+
        }

        impl ComputedValues {
            pub fn initial() -> Self {
                ComputedValues {
                    $(
                        $struct_name: ::std::rc::Rc::new(style_structs::$struct_name::initial()),
                    )+
                }
            }

            pub fn inheriting_from(parent_style: &Self) -> Self {
                ComputedValues {
                    $(
                        $struct_name: inheriting_from!($inherited $struct_name parent_style),
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
                    $($(
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
                            ::std::rc::Rc::make_mut(&mut computed.$struct_name).$ident =
                                ::style::values::ToComputedValue::to_computed(&declaration.value)
                        },
                    )+)+
                ];
                CASCADE_FNS[self.id() as usize](self, computed)
            }
        }

        type FnParseProperty =
            for<'i, 't>
            fn(&mut ::cssparser::Parser<'i, 't>)
            -> Result<PropertyDeclaration, ::style::errors::PropertyParseError<'i>>;

        ascii_case_insensitive_phf_map! {
            declaration_parsing_function_by_name -> FnParseProperty = {
                $($(
                    $name => {
                        // Using a constant works around a spurious borrow-checking error
                        // that I did not bother filing because it is fixed
                        // by MIR-based borrow-checking, so it’ll go away soon enough.
                        // FIXME: remove the indirection when NLL ships.
                        const PARSE: FnParseProperty = |parser| {
                            <$ValueType as ::style::values::Parse>::parse(parser)
                                .map(PropertyDeclaration::$ident)
                        };
                        PARSE
                    },
                )+)+
            }
        }
    }
}

macro_rules! inheriting_from {
    (inherited $struct_name: ident $parent_style: expr) => {
        $parent_style.$struct_name.clone()
    };
    (reset $struct_name: ident $parent_style: expr) => {
        ::std::rc::Rc::new(style_structs::$struct_name::initial())
    };
}
