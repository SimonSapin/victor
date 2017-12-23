//! It is valid to implement `Pod` automatically for `#[repr(C)]` structs whose fields are all `Pod`.

extern crate proc_macro;
#[macro_use] extern crate quote;
extern crate syn;

#[proc_macro_derive(Pod)]
pub fn derive_pod(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    expand_string(&input.to_string()).parse().unwrap()
}

fn expand_string(input: &str) -> String {
    let type_ = syn::parse_derive_input(input).unwrap();

    let has_repr_c = type_.attrs.iter().any(|a| match a.value {
        syn::MetaItem::List(ref name, ref contents) if name == "repr" => {
            contents.len() == 1 && match contents[0] {
                syn::NestedMetaItem::MetaItem(syn::MetaItem::Word(ref word)) => word == "C",
                _ => false
            }
        }
        _ => false,
    });
    assert!(has_repr_c, "#[derive(Pod)] requires #[repr(C)]");

    let fields = if let syn::Body::Struct(ref body) = type_.body {
        body.fields().iter().enumerate().map(|(i, f)| {
            f.ident.clone().unwrap_or_else(|| i.to_string().into())
        })
    } else {
        panic!("#[derive(Pod)] only supports structs")
    };

    let name = &type_.ident;
    let assert_fields = syn::Ident::new(format!("_assert_{}_fields_are_pod", name));
    let assert_packed = syn::Ident::new(format!("_assert_{}_repr_is_packed", name));
    let mut packed = type_.clone();
    packed.attrs = vec![syn::Attribute {
        style: syn::AttrStyle::Outer,
        value: syn::MetaItem::List("repr".into(), vec![
            syn::NestedMetaItem::MetaItem(syn::MetaItem::Word("packed".into()))
        ]),
        is_sugared_doc: false,
    }];
    packed.ident = syn::Ident::new(format!("{}Packed", name));
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
            #packed
            let _ = ::std::mem::transmute::<#name, #packed_name>;
        }
    };

    tokens.to_string()
}
