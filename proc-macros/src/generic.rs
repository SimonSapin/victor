#[proc_macro_derive(FromVariants)]
pub fn derive_from_variants(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = input.ident;
    let mut impls = quote!();
    if let syn::Data::Enum(data) = input.data {
        for variant in &data.variants {
            let variant_name = &variant.ident;
            let mut fields = variant.fields.iter();
            if let (syn::Fields::Unnamed(_), Some(field), None) =
                (&variant.fields, fields.next(), fields.next())
            {
                let ty = &field.ty;
                impls.extend(quote! {
                    impl std::convert::From<#ty> for #name {
                        fn from(x: #ty) -> Self {
                            #name :: #variant_name (x)
                        }
                    }
                })
            }
        }
    }
    if impls.is_empty() {
        panic!(
            "derive(FromVariants) requires an enum with (some) variants \
             that have a single unnamed field"
        )
    }
    impls.into()
}
