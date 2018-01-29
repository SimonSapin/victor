//! It is valid to implement `Pod` automatically for `#[repr(C)]` structs whose fields are all `Pod`.

extern crate proc_macro;
extern crate proc_macro2;
#[macro_use] extern crate quote;
extern crate syn;

use quote::ToTokens;

#[proc_macro_derive(SfntTable, attributes(tag))]
pub fn derive_sfnt_table(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let mut table_impl = quote!();
    for attr in &input.attrs {
        if let Some(syn::Meta::NameValue(ref meta)) = attr.interpret_meta() {
             if meta.ident == "tag" {
                if let syn::Lit::Str(ref tag) = meta.lit {
                    let value = tag.value();
                    assert_eq!(value.len(), 4);
                    let tag = syn::LitByteStr::new(value.as_bytes(), tag.span);
                    table_impl = quote! {
                        #[warn(dead_code)]
                        impl ::fonts::SfntTable for #name {
                            const TAG: Tag = Tag(*#tag);
                        }
                    };
                    break
                }
            }
        }
    }


    let struct_ = if let syn::Data::Struct(ref struct_) = input.data {
        struct_
    } else {
        panic!("#[derive(SfntTable)] only supports structs")
    };

    let mut methods = quote!();
    let mut offset: u32 = 0;
    for field in struct_.fields.iter() {
        let name = field.ident.as_ref().expect("Unsupported unnamed field");

        let ty = if let syn::Type::Path(ref ty) = field.ty {
            ty
        } else {
            panic!("Unsupported field type: {}", field.ty.clone().into_tokens())
        };
        assert!(ty.qself.is_none());
        let size = match ty.path.segments.last().unwrap().value().ident.as_ref() {
            "u16" | "i16" | "FWord" | "UFWord" | "FontDesignUnitsPerEmFactorU16" => 2,
            "u32" | "FixedPoint" | "Tag" => 4,
            "LongDateTime" => 8,
            _ => panic!("The size of {} is unknown", ty.clone().into_tokens())
        };
        // The TrueType format seems to be designed so that this never happens:
        let expected_align = std::cmp::min(size, 4);
        assert_eq!(offset % expected_align, 0, "Field {} is misaligned", name);
        methods.append_all(quote! {
            pub(in fonts) fn #name(self) -> ::fonts::parsing::Position<#ty> {
                self.offset_bytes(#offset)
            }
        });
        offset += size;
    }
    let size_of = offset as usize;

    let tokens = quote! {
        #table_impl

        impl #name {
            fn _assert_size_of() {
                let _ = ::std::mem::transmute::<Self, [u8; #size_of]>;
            }
        }

        #[warn(dead_code)]
        impl ::fonts::parsing::Position<#name> {
            #methods
        }
    };

    tokens.into()
}

#[proc_macro_derive(ReadFromBytes)]
pub fn derive_read_from_bytes(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: syn::DeriveInput = syn::parse(input).unwrap();
    let name = &input.ident;

    let tokens = quote! {
        impl ::fonts::parsing::ReadFromBytes for #name {
            fn read_from(bytes: &[u8]) -> Result<Self, ::fonts::FontError> {
                use fonts::parsing::ReadFromBytes;
                ReadFromBytes::read_from(bytes).map(#name)  // Assume single unnamed field
            }
        }
    };

    tokens.into()
}
