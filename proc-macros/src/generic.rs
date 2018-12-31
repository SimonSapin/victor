#[proc_macro_derive(FromVariants)]
pub fn derive_from_variants(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;

    let mut impls = quote!();
    match input.data {
        syn::Data::Enum(data) => {
            for variant in &data.variants {
                match variant.fields {
                    syn::Fields::Unnamed(_) if variant.fields.iter().len() == 1 => {
                        let variant_name = &variant.ident;
                        let ty = &variant.fields.iter().next().unwrap().ty;
                        impls.extend(quote! {
                            impl std::convert::From<#ty> for #name {
                                fn from(x: #ty) -> Self {
                                    #name :: #variant_name (x)
                                }
                            }
                        })
                    }
                    _ => {},
                }
            }
        }
        _ => unimplemented!(),
    };
    impls.into()
}
