use proc_macro::TokenStream;

#[proc_macro_derive(SpecifiedAsComputed)]
pub fn derive_specified_as_computed(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    quote!(
        impl crate::style::values::SpecifiedValue for #name {
            type SpecifiedValue = Self;
        }
        impl crate::style::values::EarlyFromSpecified for #name {
            fn early_from_specified(
                specified: &Self,
                _context: &crate::style::values::EarlyCascadeContext,
            ) -> Self {
                std::clone::Clone::clone(specified)
            }
        }
        impl crate::style::values::FromSpecified for #name {
            fn from_specified(
                specified: &Self,
                _context: &crate::style::values::CascadeContext,
            ) -> Self {
                std::clone::Clone::clone(specified)
            }
        }
    )
    .into()
}

#[proc_macro_derive(FromSpecified)]
pub fn derive_from_specified(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;
    let (specified_type, specified_name) = if input.generics.type_params().next().is_none() {
        let specified = syn::Ident::new(&format!("Specified{}", name), name.span());
        (quote!(#specified), specified)
    } else {
        let type_params = input.generics.type_params().map(|p| &p.ident);
        (
            quote!(#name< #( #type_params::SpecifiedValue ),* >),
            name.clone(),
        )
    };
    let gen_variant = |fields, variant| match fields {
        &syn::Fields::Unit => quote! {
            #specified_name #variant => #name  #variant,
        },
        &syn::Fields::Unnamed(_) => {
            let fields = &fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    syn::Ident::new(&format!("f{}", i), syn::spanned::Spanned::span(field))
                })
                .collect::<Vec<_>>();
            quote! {
                #specified_name #variant ( #( #fields ),* ) => #name #variant (
                    #(
                        FromSpecified::from_specified(#fields, context),
                    )*
                ),
            }
        }
        &syn::Fields::Named(_) => {
            let fields = &fields
                .iter()
                .map(|field| field.ident.as_ref().unwrap())
                .collect::<Vec<_>>();
            let fields2 = fields;
            quote! {
                #specified_name #variant { #( #fields ),* } => #name #variant {
                    #(
                        #fields: FromSpecified::from_specified(#fields2, context),
                    )*
                },
            }
        }
    };
    let variants = match &input.data {
        syn::Data::Struct(data) => vec![gen_variant(&data.fields, quote!())],
        syn::Data::Enum(data) => data
            .variants
            .iter()
            .map(|variant| {
                let variant_name = &variant.ident;
                gen_variant(&variant.fields, quote!(:: #variant_name))
            })
            .collect(),
        syn::Data::Union(_) => unimplemented!(),
    };
    let a = derive_trait(
        &input,
        quote!(crate::style::values::SpecifiedValue),
        quote!(type SpecifiedValue = #specified_type;),
    );
    let b = derive_trait(
        &input,
        quote!(crate::style::values::FromSpecified),
        quote! {
            fn from_specified(
                specified: &Self::SpecifiedValue,
                context: &crate::style::values::CascadeContext,
            ) -> Self {
                use crate::style::values::FromSpecified;
                match specified {
                    #( #variants )*
                }
            }
        },
    );
    quote!(#a #b).into()
}

#[proc_macro_derive(Parse)]
pub fn derive_parse(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let mut unit_variants = Vec::new();
    let mut keywords = Vec::new();
    let mut single_unnamed_field_variants = Vec::new();

    match &input.data {
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                match variant.fields {
                    syn::Fields::Unit => {
                        let variant = &variant.ident;
                        unit_variants.push(quote!(#name :: #variant));
                        keywords.push(camel_case_to_kebab_case(&variant.to_string()))
                    }
                    syn::Fields::Unnamed(_) if variant.fields.iter().len() == 1 => {
                        let variant = &variant.ident;
                        single_unnamed_field_variants.push(quote!(#name :: #variant))
                    }
                    _ => unimplemented!(),
                }
            }
        }
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Unit,
            ..
        }) => {
            unit_variants.push(quote!(#name));
            keywords.push(camel_case_to_kebab_case(&name.to_string()))
        }
        _ => panic!("derive(Parse) only supports enums"),
    };

    let mut parse = quote! {
        #(
            if let Ok(value) = parser.r#try(crate::style::values::Parse::parse) {
                return Ok(#single_unnamed_field_variants(value))
            }
        )*
    };
    if !unit_variants.is_empty() {
        parse.extend(quote! {
            if let Ok(ident) = parser.r#try(|parser| parser.expect_ident_cloned()) {
                match &*ident {
                    #(
                        #keywords => return Ok(#unit_variants),
                    )*
                    _ => return Err(parser.new_unexpected_token_error(
                        cssparser::Token::Ident(ident)
                    ))
                }
            }
        })
    }
    derive_trait(
        &input,
        quote!(crate::style::values::Parse),
        quote! {
            fn parse<'i, 't>(parser: &mut cssparser::Parser<'i, 't>)
                -> Result<Self, crate::style::errors::PropertyParseError<'i>>
            {
                #parse
                Err(parser.new_error_for_next_token())
            }
        },
    )
    .into()
}

fn camel_case_to_kebab_case(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_string().chars() {
        if c.is_ascii_lowercase() {
            out.push(c)
        } else if c.is_ascii_uppercase() {
            if !out.is_empty() {
                out.push('-')
            }
            out.push(c.to_ascii_lowercase())
        } else {
            panic!(
                "Unsupported char {:?}, converting {:?} from CamelCase",
                c, s
            )
        }
    }
    out
}

fn derive_trait(
    input: &syn::DeriveInput,
    trait_: proc_macro2::TokenStream,
    items: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let type_ = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let predicates = where_clause.map(|w| &w.predicates);
    let additional_predicates = input
        .generics
        .type_params()
        .map(|p| &p.ident)
        .map(|p| quote! { #p: #trait_, })
        .collect::<Vec<_>>();
    quote! {
        impl #impl_generics #trait_ for #type_ #ty_generics
            where
                #( #additional_predicates )*
                #predicates
        {
            #items
        }
    }
}
