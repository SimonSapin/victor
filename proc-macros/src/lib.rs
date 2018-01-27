//! It is valid to implement `Pod` automatically for `#[repr(C)]` structs whose fields are all `Pod`.

extern crate proc_macro;
#[macro_use] extern crate quote;
extern crate syn;

use quote::ToTokens;

#[proc_macro_derive(SfntTable)]
pub fn derive_sfnt_table(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let struct_ = if let syn::Data::Struct(ref struct_) = input.data {
        struct_
    } else {
        panic!("#[derive(SfntTable)] only supports structs")
    };

    let mut methods = quote!();
    let mut offset: usize = 0;
    for field in struct_.fields.iter() {
        let name = field.ident.as_ref().expect("Unsupported unnamed field");

        let ty = if let syn::Type::Path(ref ty) = field.ty {
            ty
        } else {
            panic!("Unsupported field type: {}", field.ty.clone().into_tokens())
        };
        assert!(ty.qself.is_none());
        let size = match ty.path.segments.last().unwrap().value().ident.as_ref() {
            "u16" => 2,
            "u32" | "Tag" => 4,
            _ => panic!("The size of {} is unknown", ty.clone().into_tokens())
        };
        // The TrueType format seems to be designed so that this never happens:
        assert_eq!(offset % size, 0, "Field {} is misaligned", name);
        methods.append_all(quote! {
            pub(in fonts2) fn #name(self) -> Position<#ty> {
                self.offset(#offset)
            }
        });
        offset += size;
    }
    let size_of = offset;

    let name = &input.ident;
    let tokens = quote! {
        impl #name {
            fn _assert_size_of() {
                let _ = ::std::mem::transmute::<Self, [u8; #size_of]>;
            }
        }

        #[warn(dead_code)]
        impl Position<#name> {
            #methods
        }
    };

    tokens.into()
}

#[proc_macro_derive(Pod)]
pub fn derive_pod(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();

    let has_repr_c = input.attrs.iter().any(|a| match a.interpret_meta() {
        Some(syn::Meta::List(ref list)) if list.ident == "repr" => {
            list.nested.len() == 1 && match list.nested[0] {
                syn::NestedMeta::Meta(syn::Meta::Word(ref word)) => word == "C",
                _ => false
            }
        }
        _ => false,
    });
    assert!(has_repr_c, "#[derive(Pod)] requires #[repr(C)]");

    let fields = if let syn::Data::Struct(ref struct_) = input.data {
        struct_.fields.iter().enumerate().map(|(i, f)| {
            if let Some(ref ident) = f.ident {
                syn::Member::Named(ident.clone())
            } else {
                syn::Member::Unnamed(syn::Index::from(i))
            }
        })
    } else {
        panic!("#[derive(Pod)] only supports structs")
    };

    let name = &input.ident;
    let assert_fields = syn::Ident::from(format!("_assert_{}_fields_are_pod", name));
    let assert_packed = syn::Ident::from(format!("_assert_{}_repr_is_packed", name));
    let mut packed = input.clone();
    packed.attrs.clear();
    packed.ident = syn::Ident::from(format!("{}Packed", name));
    let packed_name = &packed.ident;

    let tokens = quote! {
        unsafe impl Pod for #name {}

        #[allow(non_snake_case, unused_variables)]
        fn #assert_fields(value: &#name) {
            fn _assert_pod<T: Pod>(_: &T) {
            }
            #(
                _assert_pod(&value.#fields);
            )*
        }

        #[allow(non_snake_case, non_camel_case_types)]
        fn #assert_packed() {
            #[repr(packed)]
            #packed
            let _ = ::std::mem::transmute::<#name, #packed_name>;
        }
    };

    tokens.into()
}
