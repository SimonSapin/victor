use crate::style::errors::PropertyParseError;
use crate::style::values::{Parse, ToComputedValue};
use cssparser::Parser;
use std::rc::Rc;

macro_rules! properties {
    (
        type Discriminant = $DiscriminantType: ident;
        $(
            $inherited: ident struct $struct_name: ident {
                $(
                    $ident: ident {
                        $name: expr,
                        $ValueType: ty,
                        initial = $initial_value: expr
                    }
                )+
            }
        )+
        @shorthands {
            $(
                $shorthand_name: tt => $shorthand_struct: ident {
                    $(
                        $shorthand_field: ident: $longhand_ident: ident,
                    )+
                }
            )+
        }
    ) => {
        #[repr($DiscriminantType)]
        #[allow(non_camel_case_types)]
        pub enum LonghandDeclaration {
            $($(
                $ident($ValueType),
            )+)+
        }

        #[derive(Clone)]
        pub struct ComputedValues {
            $(
                pub $struct_name: Rc<style_structs::$struct_name>,
            )+
        }

        pub mod style_structs {
            use super::*;
            $(
                #[allow(non_camel_case_types)]
                #[derive(Clone)]  // FIXME: only for inherited structs?
                pub struct $struct_name {
                    $(
                        pub $ident: <$ValueType as ToComputedValue>::Computed,
                    )+
                }
            )+
        }

        // XXX: if we ever replace Rc with Arc for style structs,
        // replace thread_local! with lazy_static! here.
        thread_local! {
            static INITIAL_VALUES: Rc<ComputedValues> = Rc::new(ComputedValues {
                $(
                    $struct_name: Rc::new(
                        style_structs::$struct_name {
                            $(
                                $ident: $initial_value,
                            )+
                        }
                    ),
                )+
            });
        }

        impl ComputedValues {
            pub fn initial() -> Rc<Self> {
                INITIAL_VALUES.with(|initial| initial.clone())
            }

            pub fn new_inheriting_from(inherited: &Self, initial: &Self) -> Self {
                macro_rules! select {
                    (inherited) => { inherited };
                    (reset) => { initial };
                }
                ComputedValues {
                    $(
                        $struct_name: Rc::clone(&select!($inherited).$struct_name),
                    )+
                }
        }

            pub fn anonymous_inheriting_from(parent_style: &Self) -> Rc<Self> {
                INITIAL_VALUES.with(|initial| {
                    Rc::new(Self::new_inheriting_from(parent_style, initial))
                })
            }
        }

        impl LonghandDeclaration {
            fn id(&self) -> $DiscriminantType {
                // #[repr(u8)] guarantees that an enum’s representation starts with a u8 tag:
                // https://rust-lang.github.io/rfcs/2195-really-tagged-unions.html
                let ptr: *const LonghandDeclaration = self;
                let ptr = ptr as *const $DiscriminantType;
                unsafe {
                    *ptr
                }
            }

            pub fn cascade_into(
                &self,
                computed: &mut ComputedValues,
                _inherited: &ComputedValues,
            ) {
                static CASCADE_FNS: &'static [fn(&LonghandDeclaration, &mut ComputedValues)] = &[
                    $($(
                        |declaration, computed| {
                            // https://rust-lang.github.io/rfcs/2195-really-tagged-unions.html
                            #[repr(C)]
                            struct Repr {
                                tag: $DiscriminantType,
                                value: $ValueType,
                            }
                            let ptr: *const LonghandDeclaration = declaration;
                            let ptr = ptr as *const Repr;
                            let declaration = unsafe {
                                &*ptr
                            };
                            Rc::make_mut(&mut computed.$struct_name).$ident =
                                ToComputedValue::to_computed(&declaration.value)
                        },
                    )+)+
                ];
                CASCADE_FNS[self.id() as usize](self, computed)
            }
        }

        type FnParseProperty =
            for<'i, 't>
            fn(&mut Parser<'i, 't>, &mut Vec<LonghandDeclaration>)
            -> Result<(), PropertyParseError<'i>>;

        ascii_case_insensitive_phf_map! {
            declaration_parsing_function_by_name -> FnParseProperty = {
                $($(
                    $name => |parser, declarations| {
                        let v = Parse::parse(parser)?;
                        declarations.push(LonghandDeclaration::$ident(v));
                        Ok(())
                    },
                )+)+
                $(
                    $shorthand_name => |parser, declarations| {
                        let $shorthand_struct {
                            $(
                                $shorthand_field: $longhand_ident,
                            )+
                        } = Parse::parse(parser)?;
                        $(
                            declarations.push(
                                LonghandDeclaration::$longhand_ident($longhand_ident)
                            );
                        )+
                        Ok(())
                    },
                )+
            }
        }
    }
}
