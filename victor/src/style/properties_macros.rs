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
        pub enum PropertyDeclaration {
            $($(
                $ident($ValueType),
            )+)+
        }

        #[derive(Clone)]
        pub struct ComputedValues {
            $(
                pub $struct_name: ::std::rc::Rc<style_structs::$struct_name>,
            )+
        }

        pub mod style_structs {
            use super::*;
            $(
                #[allow(non_camel_case_types)]
                #[derive(Clone)]  // FIXME: only for inherited structs?
                pub struct $struct_name {
                    $(
                        pub $ident: <$ValueType as ::style::values::ToComputedValue>::Computed,
                    )+
                }
            )+
        }

        impl ComputedValues {
            pub fn new(parent_style: Option<&Self>) -> Self {
                // XXX: if we ever replace Rc with Arc for style structs,
                // replace thread_local! with lazy_static! here.
                thread_local! {
                    static INITIAL_VALUES: ComputedValues = ComputedValues {
                        $(
                            $struct_name: ::std::rc::Rc::new(
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
                                $struct_name: ::std::rc::Rc::clone(
                                    &select!($inherited, parent, initial).$struct_name
                                ),
                            )+
                        }
                    } else {
                        initial.clone()
                    }
                })
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
            fn(&mut ::cssparser::Parser<'i, 't>, &mut Vec<PropertyDeclaration>)
            -> Result<(), ::style::errors::PropertyParseError<'i>>;

        ascii_case_insensitive_phf_map! {
            declaration_parsing_function_by_name -> FnParseProperty = {
                $($(
                    $name => {
                        // Using a constant works around a spurious borrow-checking error
                        // that I did not bother filing because it is fixed
                        // by MIR-based borrow-checking, so it’ll go away soon enough.
                        // FIXME: remove the indirection when NLL ships.
                        const PARSE: FnParseProperty = |parser, declarations| {
                            let v = <$ValueType as ::style::values::Parse>::parse(parser)?;
                            declarations.push(PropertyDeclaration::$ident(v));
                            Ok(())
                        };
                        PARSE
                    },
                )+)+
                $(
                    $shorthand_name => {
                        const PARSE: FnParseProperty = $shorthand_parse;
                        PARSE
                    },
                )+
            }
        }
    }
}

macro_rules! parse_four_sides {
    ($Top: ident, $Left: ident, $Bottom: ident, $Right: ident) => {
        |parser, declarations: &mut Vec<PropertyDeclaration>| {
            let FourSides { top, left, bottom, right } = <FourSides<_> as Parse>::parse(parser)?;
            declarations.push(PropertyDeclaration::$Top(top));
            declarations.push(PropertyDeclaration::$Left(left));
            declarations.push(PropertyDeclaration::$Bottom(bottom));
            declarations.push(PropertyDeclaration::$Right(right));
            Ok(())
        }
    }
}
