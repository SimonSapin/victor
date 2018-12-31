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

        tagged_union_with_jump_tables! {
            #[repr($DiscriminantType)]
            #[allow(non_camel_case_types)]
            pub(in crate::style) enum LonghandDeclaration {
                $($(
                    $ident(<$ValueType as crate::style::values::FromSpecified>::SpecifiedValue),
                )+)+
                CssWide(LonghandId, crate::style::values::CssWideKeyword)
            }

            pub(in crate::style) fn cascade_into(
                &self,
                computed: &mut ComputedValues,
                inherited: &ComputedValues,
            ) {
                match *self {
                    $($(
                        LonghandDeclaration::$ident(ref value) => {
                            Rc::make_mut(&mut computed.$struct_name).$ident =
                                crate::style::values::FromSpecified::from_specified(value)
                        }
                    )+)+
                    LonghandDeclaration::CssWide(ref longhand, ref keyword) => {
                        longhand.cascade_css_wide_keyword_into(*keyword, computed, inherited)
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

        // XXX: if we ever replace Rc with Arc for style structs,
        // replace thread_local! with lazy_static! here.
        thread_local! {
            static INITIAL_VALUES: Rc<ComputedValues> = Rc::new(ComputedValues {
                $(
                    $struct_name: Rc::new(
                        style_structs::$struct_name {
                            $(
                                $ident: From::from($initial_value),
                            )+
                        }
                    ),
                )+
            });
        }

        impl ComputedValues {
            pub(crate) fn new_inheriting_from(inherited: &Self, initial: &Self) -> Self {
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
        }

        ascii_case_insensitive_phf_map! {
            property_data_by_name -> PropertyData = {
                $($(
                    $name => PropertyData {
                        longhands: &[LonghandId::$ident],
                        parse: |parser, declarations| {
                            let v = crate::style::values::Parse::parse(parser)?;
                            declarations.push(LonghandDeclaration::$ident(v));
                            Ok(())
                        },
                    },
                )+)+
                $(
                    $shorthand_name => PropertyData {
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
                                    ValueOrInitial::into(
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

impl ComputedValues {
    pub(crate) fn initial() -> Rc<Self> {
        INITIAL_VALUES.with(|initial| initial.clone())
    }

    pub(crate) fn anonymous_inheriting_from(parent_style: &Self) -> Rc<Self> {
        INITIAL_VALUES.with(|initial| Rc::new(Self::new_inheriting_from(parent_style, initial)))
    }
}

type FnParseProperty = for<'i, 't> fn(
    &mut cssparser::Parser<'i, 't>,
    &mut Vec<LonghandDeclaration>,
) -> Result<(), crate::style::errors::PropertyParseError<'i>>;

pub struct PropertyData {
    pub(in crate::style) longhands: &'static [LonghandId],
    pub(in crate::style) parse: FnParseProperty,
}

trait ValueOrInitial<T> {
    fn into<F>(self, id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration;
}

impl<T> ValueOrInitial<T> for T {
    fn into<F>(self, _id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration,
    {
        constructor(self)
    }
}

impl<T> ValueOrInitial<T> for Option<T> {
    fn into<F>(self, id: LonghandId, constructor: F) -> LonghandDeclaration
    where
        F: Fn(T) -> LonghandDeclaration,
    {
        use crate::style::values::CssWideKeyword;
        match self {
            Some(value) => constructor(value),
            None => LonghandDeclaration::CssWide(id, CssWideKeyword::Initial),
        }
    }
}
