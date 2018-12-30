#[proc_macro_derive(SpecifiedAsComputed)]
pub fn derive_specified_as_computed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let tokens = quote! {
        impl crate::style::values::FromSpecified for #name {
            type SpecifiedValue = Self;
            fn from_specified(specified: &Self) -> Self {
                std::clone::Clone::clone(specified)
            }
        }
    };

    tokens.into()
}

#[proc_macro_derive(Parse)]
pub fn derive_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

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
        .map(|ident| {
            let mut name = String::new();
            for c in ident.to_string().chars() {
                if c.is_ascii_lowercase() {
                    name.push(c)
                } else if c.is_ascii_uppercase() {
                    if !name.is_empty() {
                        name.push('-')
                    }
                    name.push(c.to_ascii_lowercase())
                } else {
                    panic!("Unsupported variant name char {:?}", c)
                }
            }
            name
        })
        .collect();

    let tokens = quote! {
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
    };

    tokens.into()
}
