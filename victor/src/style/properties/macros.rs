macro_rules! properties {
    (
        type Discriminant = $DiscriminantType: ident;
        $(
            $inherited: ident struct $struct_name: ident {
                $(
                    $( @$early: ident )? $ident: ident {
                        $name: expr,
                        $ValueType: ty,
                        initial = $initial_value: expr
                    }
                )+
            }
        )+
        @shorthands {
            $(
                $shorthand_name: tt => $shorthand_struct: path {
                    $(
                        $shorthand_field: ident: $longhand_ident: ident,
                    )+
                }
            )+
        }
    ) => {
        use std::rc::Rc;

        tagged_union_with_jump_tables! {
            #[repr($DiscriminantType)]
            #[derive(Copy, Clone)]
            #[allow(non_camel_case_types)]
            pub(in crate::style) enum LonghandId {
                $($(
                    $ident,
                )+)+
            }

            fn cascade_css_wide_keyword_into(
                &self,
                keyword: crate::style::values::CssWideKeyword,
                computed: &mut ComputedValues,
                inherited: &ComputedValues,
            ) {
                match *self {
                    $($(
                        LonghandId::$ident => {
                            macro_rules! unset_is_initial {
                                (inherited) => { false };
                                (reset) => { true };
                            }
                            use crate::style::values::CssWideKeyword;
                            let is_initial = match keyword {
                                CssWideKeyword::Initial => true,
                                CssWideKeyword::Inherit => false,
                                CssWideKeyword::Unset => unset_is_initial!($inherited),
                            };
                            Rc::make_mut(&mut computed.$struct_name).$ident =
                            if is_initial {
                                From::from($initial_value)
                            } else {
                                inherited.$struct_name.$ident.clone()
                            };
                        }
                    )+)+
                }
            }
        }

        impl LonghandId {
            fn is_early(self) -> bool {
                macro_rules! is_early {
                    (early) => { true };
                    () => { false };
                }
                [
                    $($(
                        is_early!($( $early )?),
                    )+)+
                ][self as usize]
            }
        }

        macro_rules! if_early {
            (early $then: block) => { $then };
            ($then: block) => {};
        }

        macro_rules! if_late {
            (early $then: block) => {};
            ($then: block) => { $then };
        }

        tagged_union_with_jump_tables! {
            #[repr($DiscriminantType)]
            #[allow(non_camel_case_types)]
            pub(in crate::style) enum LonghandDeclaration {
                $($(
                    $ident(<$ValueType as crate::style::values::SpecifiedValue>::SpecifiedValue),
                )+)+
                CssWide(LonghandId, crate::style::values::CssWideKeyword)
            }

            pub(in crate::style) fn if_early_cascade_into(
                &self,
                computed: &mut ComputedValues,
                context: &crate::style::values::EarlyCascadeContext,
            ) {
                match *self {
                    $($(
                        LonghandDeclaration::$ident(ref _value) => {
                            if_early! {
                                $( $early )? {
                                    Rc::make_mut(&mut computed.$struct_name).$ident =
                                        crate::style::values::EarlyFromSpecified::early_from_specified(_value, context)
                                }
                            }
                        }
                    )+)+
                    LonghandDeclaration::CssWide(ref longhand, ref keyword) => {
                        if longhand.is_early() {
                            longhand.cascade_css_wide_keyword_into(*keyword, computed, context.inherited)
                        }
                    }
                }
            }

            pub(in crate::style) fn if_late_cascade_into(
                &self,
                computed: &mut ComputedValues,
                context: &crate::style::values::CascadeContext,
            ) {
                match *self {
                    $($(
                        LonghandDeclaration::$ident(ref _value) => {
                            if_late! {
                                $( $early )? {
                                    Rc::make_mut(&mut computed.$struct_name).$ident =
                                        crate::style::values::FromSpecified::from_specified(_value, context)
                                }
                            }
                        }
                    )+)+
                    LonghandDeclaration::CssWide(ref longhand, ref keyword) => {
                        if !longhand.is_early() {
                            longhand.cascade_css_wide_keyword_into(*keyword, computed, context.inherited)
                        }
                    }
                }
            }
        }

        #[derive(Clone)]
        pub(crate) struct ComputedValues {
            $(
                pub(crate) $struct_name: Rc<style_structs::$struct_name>,
            )+
        }

        pub(crate) mod style_structs {
            use super::*;
            $(
                #[allow(non_camel_case_types)]
                #[derive(Clone)]  // FIXME: only for inherited structs?
                pub(crate) struct $struct_name {
                    $(
                        pub(crate) $ident: $ValueType,
                    )+
                }
            )+
        }

        impl ComputedValues {
            pub(in crate::style) fn new(
                inherited: Option<&Self>,
                matching: Option<&crate::style::cascade_module::MatchingDeclarations>,
            ) -> Rc<Self> {
                // XXX: if we ever replace Rc with Arc for style structs,
                // replace thread_local! with lazy_static! here.
                thread_local! {
                    static INITIAL_VALUES: ComputedValues = ComputedValues {
                        $(
                            $struct_name: Rc::new(
                                style_structs::$struct_name {
                                    $(
                                        $ident: From::from($initial_value),
                                    )+
                                }
                            ),
                        )+
                    };
                }
                INITIAL_VALUES.with(|initial| {
                    let inherited = inherited.unwrap_or(initial);
                    macro_rules! select {
                        (inherited) => { inherited };
                        (reset) => { initial };
                    }
                    let mut computed = ComputedValues {
                        $(
                            $struct_name: Rc::clone(&select!($inherited).$struct_name),
                        )+
                    };
                    if let Some(matching) = matching {
                        let context = crate::style::values::EarlyCascadeContext {
                            inherited,
                        };
                        matching.for_each(&mut |decl| {
                            decl.if_early_cascade_into(&mut computed, &context)
                        });
                        let context = crate::style::values::CascadeContext {
                            inherited,
                        };
                        matching.for_each(&mut |decl| {
                            decl.if_late_cascade_into(&mut computed, &context)
                        });
                    }
                    computed.post_cascade_fixups();
                    Rc::new(computed)
                })
            }
        }

        ascii_case_insensitive_phf_map! {
            property_data_by_name -> crate::style::properties::PropertyData = {
                $($(
                    $name => crate::style::properties::PropertyData {
                        longhands: &[LonghandId::$ident],
                        parse: |parser, declarations| {
                            let v = crate::style::values::Parse::parse(parser)?;
                            declarations.push(LonghandDeclaration::$ident(v));
                            Ok(())
                        },
                    },
                )+)+
                $(
                    $shorthand_name => crate::style::properties::PropertyData {
                        longhands: &[
                            $(
                                LonghandId::$longhand_ident,
                            )+
                        ],
                        parse: |parser, declarations| {
                            let $shorthand_struct {
                                $(
                                    $shorthand_field: $longhand_ident,
                                )+
                            } = crate::style::values::Parse::parse(parser)?;
                            $(
                                declarations.push(
                                    crate::style::properties::ValueOrInitial::into(
                                        $longhand_ident,
                                        LonghandId::$longhand_ident,
                                        LonghandDeclaration::$longhand_ident,
                                    )
                                );
                            )+
                            Ok(())
                        },
                    },
                )+
            }
        }

    }
}
