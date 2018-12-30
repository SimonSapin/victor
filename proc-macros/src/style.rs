#[proc_macro_derive(SpecifiedAsComputed)]
pub fn derive_specified_as_computed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    quote!(
        impl crate::style::values::FromSpecified for #name {
            type SpecifiedValue = Self;
            fn from_specified(specified: &Self) -> Self {
                std::clone::Clone::clone(specified)
            }
        }
    )
    .into()
}

#[proc_macro_derive(FromSpecified)]
pub fn derive_from_specified(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    let specified = syn::Ident::new(&format!("Specified{}", name), name.span());
    let gen_variant = |fields, variant| match fields {
        syn::Fields::Unit => quote! {
            #specified #variant => #name  #variant,
        },
        syn::Fields::Unnamed(_) => {
            let fields = &fields
                .iter()
                .enumerate()
                .map(|(i, field)| {
                    syn::Ident::new(&format!("f{}", i), syn::spanned::Spanned::span(field))
                })
                .collect::<Vec<_>>();
            quote! {
                #specified #variant ( #( #fields ),* ) => #name #variant (
                    #(
                        FromSpecified::from_specified(#fields),
                    )*
                ),
            }
        }
        syn::Fields::Named(_) => {
            let fields = &fields
                .iter()
                .map(|field| field.ident.as_ref().unwrap())
                .collect::<Vec<_>>();
            let fields2 = fields;
            quote! {
                #specified #variant { #( #fields ),* } => #name #variant {
                    #(
                        #fields: FromSpecified::from_specified(#fields2),
                    )*
                },
            }
        }
    };
    let variants = match input.data {
        syn::Data::Struct(data) => vec![gen_variant(data.fields, quote!())],
        syn::Data::Enum(data) => data
            .variants
            .into_iter()
            .map(|variant| {
                let variant_name = variant.ident;
                gen_variant(variant.fields, quote!(:: #variant_name))
            })
            .collect(),
        syn::Data::Union(_) => unimplemented!(),
    };
    quote!(
        impl crate::style::values::FromSpecified for #name {
            type SpecifiedValue = #specified;
            fn from_specified(specified: &Self::SpecifiedValue) -> Self {
                use crate::style::values::FromSpecified;
                match specified {
                    #( #variants )*
                }
            }
        }
    )
    .into()
}

#[proc_macro_derive(Parse)]
pub fn derive_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;

    let variants: Vec<_> = match input.data {
        syn::Data::Enum(data) => data
            .variants
            .into_iter()
            .map(|variant| variant.ident)
            .collect(),
        _ => panic!("derive(Parse) only supports enums"),
    };

    let names: Vec<_> = variants
        .iter()
        .map(|ident| camel_case_to_kebab_case(&ident.to_string()))
        .collect();

    quote!(
        impl crate::style::values::Parse for #name {
            fn parse<'i, 't>(parser: &mut cssparser::Parser<'i, 't>)
                -> Result<Self, crate::style::errors::PropertyParseError<'i>>
            {
                use self::#name::*;
                let ident = parser.expect_ident()?;
                match &**ident {
                    #(
                        #names => Ok(#variants),
                    )*
                    _ => {
                        let token = cssparser::Token::Ident(ident.clone());
                        Err(parser.new_unexpected_token_error(token))
                    }
                }
            }
        }
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
