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
                $shorthand_name: tt : $shorthand_parse: expr;
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

        impl ComputedValues {
            pub fn new_inheriting_from(parent_style: Option<&Self>) -> Self {
                // XXX: if we ever replace Rc with Arc for style structs,
                // replace thread_local! with lazy_static! here.
                thread_local! {
                    static INITIAL_VALUES: ComputedValues = ComputedValues {
                        $(
                            $struct_name: Rc::new(
                                style_structs::$struct_name {
                                    $(
                                        $ident: $initial_value,
                                    )+
                                }
                            ),
                        )+
                    };
                }

                INITIAL_VALUES.with(|initial| {
                    if let Some(parent) = parent_style {
                        macro_rules! select {
                            (inherited, $parent: expr, $initial: expr) => { $parent };
                            (reset,     $parent: expr, $initial: expr) => { $initial };
                        }
                        ComputedValues {
                            $(
                                $struct_name: Rc::clone(
                                    &select!($inherited, parent, initial).$struct_name
                                ),
                            )+
                        }
                    } else {
                        initial.clone()
                    }
                })
            }

            pub fn anonymous_inheriting_from(parent_style: &Self) -> Rc<Self> {
                Rc::new(Self::new_inheriting_from(Some(parent_style)))
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

            pub fn cascade_into(&self, computed: &mut ComputedValues) {
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
                    $shorthand_name => $shorthand_parse,
                )+
            }
        }
    }
}

macro_rules! parse_four_sides {
    ($Top: ident, $Left: ident, $Bottom: ident, $Right: ident) => {
        |parser, declarations: &mut Vec<LonghandDeclaration>| {
            let FourSides {
                top,
                left,
                bottom,
                right,
            } = Parse::parse(parser)?;
            declarations.push(LonghandDeclaration::$Top(top));
            declarations.push(LonghandDeclaration::$Left(left));
            declarations.push(LonghandDeclaration::$Bottom(bottom));
            declarations.push(LonghandDeclaration::$Right(right));
            Ok(())
        }
    };
}
