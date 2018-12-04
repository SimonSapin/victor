use crate::style::errors::PropertyParseError;
use crate::style::values::{CssWideKeyword, Parse, ToComputedValue};
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
                $shorthand_name: tt => $shorthand_struct: path {
                    $(
                        $shorthand_field: ident: $longhand_ident: ident,
                    )+
                }
            )+
        }
    ) => {
        tagged_union_with_jump_tables! {
            #[repr($DiscriminantType)]
            #[derive(Copy, Clone)]
            #[allow(non_camel_case_types)]
            pub enum LonghandId {
                $($(
                    $ident,
                )+)+
            }

            pub fn cascade_css_wide_keyword_into(
                &self,
                keyword: CssWideKeyword,
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
                            let is_initial = match keyword {
                                CssWideKeyword::Initial => true,
                                CssWideKeyword::Inherit => false,
                                CssWideKeyword::Unset => unset_is_initial!($inherited),
                            };
                            Rc::make_mut(&mut computed.$struct_name).$ident =
                            if is_initial {
                                $initial_value
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
            pub enum LonghandDeclaration {
                $($(
                    $ident($ValueType),
                )+)+
                CssWide(LonghandId, CssWideKeyword)
            }

            pub fn cascade_into(
                &self,
                computed: &mut ComputedValues,
                inherited: &ComputedValues,
            ) {
                match *self {
                    $($(
                        LonghandDeclaration::$ident(ref value) => {
                            Rc::make_mut(&mut computed.$struct_name).$ident =
                                ToComputedValue::to_computed(value)
                        }
                    )+)+
                    LonghandDeclaration::CssWide(ref longhand, ref keyword) => {
                        longhand.cascade_css_wide_keyword_into(*keyword, computed, inherited)
                    }
                }
            }
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
        }

        ascii_case_insensitive_phf_map! {
            property_data_by_name -> PropertyData = {
                $($(
                    $name => PropertyData {
                        longhands: &[LonghandId::$ident],
                        parse: |parser, declarations| {
                            let v = Parse::parse(parser)?;
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
                            } = Parse::parse(parser)?;
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
    pub fn initial() -> Rc<Self> {
        INITIAL_VALUES.with(|initial| initial.clone())
    }

    pub fn anonymous_inheriting_from(parent_style: &Self) -> Rc<Self> {
        INITIAL_VALUES.with(|initial| Rc::new(Self::new_inheriting_from(parent_style, initial)))
    }
}

type FnParseProperty = for<'i, 't> fn(
    &mut Parser<'i, 't>,
    &mut Vec<LonghandDeclaration>,
) -> Result<(), PropertyParseError<'i>>;

pub struct PropertyData {
    pub longhands: &'static [LonghandId],
    pub parse: FnParseProperty,
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
        match self {
            Some(value) => constructor(value),
            None => LonghandDeclaration::CssWide(id, CssWideKeyword::Initial),
        }
    }
}
