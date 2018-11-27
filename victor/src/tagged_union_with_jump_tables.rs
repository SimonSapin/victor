// FIXME use `?` zero-or-one repetition when it is stable
// https://github.com/rust-lang/rust/issues/48075

/// Create an enum with methods that match on it,
/// where matching is implemented as a jump table
/// (static array of function pointers, indexed by discriminant).
///
/// This is based on RFC 2195:
/// <https://rust-lang.github.io/rfcs/2195-really-tagged-unions.html>
///
/// See usage example in test at the end of this file
#[macro_export]
macro_rules! tagged_union_with_jump_tables {
    (
        #[repr($discriminant_type: ident)]
        $( #[$attr: meta] )*
        $visibility: vis enum $EnumName: ident {
            $(
                $Variant: ident $(
                    (
                        $(
                            $variant_field_type: ty
                        ),*
                        $(,)*  // FIXME #48075
                    )
                )*  // FIXME #48075
            ),*
            $(,)*  // FIXME #48075
        }

        $($tail: tt)*
    ) => {
        tagged_union_with_jump_tables!(@assert_ident_is_int $discriminant_type);
        #[repr($discriminant_type)]
        $( #[$attr] )*
        $visibility enum $EnumName {
            $(
                $Variant $( ($($variant_field_type),*) )*,
            )*
        }

        impl $EnumName {
            tagged_union_with_jump_tables! {
                @methods
                $discriminant_type
                $EnumName { $( $Variant $( ($( $variant_field_type )*) )* )* }
                $($tail)*
            }
        }
    };
    (
        @methods
        $discriminant_type: ident
        $EnumName: ident { $( $Variant: ident $( ($( $variant_field_type: ty )*) )* )* }

        $visibility: vis fn $method: ident(
            &self
            $( , $arg: ident: $arg_type: ty)*
            $(,)*  // FIXME #48075
        ) -> $ret: ty  {
            match *self {
                $(
                    $ReEnumName: ident::$MatchedVariant: ident $( (
                        $(ref $matched_field: ident),* $(,)*
                    ) )*  // FIXME #48075
                    =>  $block: block
                )*
            }
        }

        $($tail: tt)*
    ) => {
        $visibility fn $method(&self $(, $arg: $arg_type)*) -> $ret {
            // The layout of an enum with #[repr($Int)] always starts
            // with the discriminant, an integer tag of type $Int
            #[repr(C)]
            struct AnyVariant {
                tag: $discriminant_type,
                // Other fields vary per variant
            }
            let ptr: *const $EnumName = self;
            let ptr = ptr as *const AnyVariant;
            let tag = unsafe { (*ptr).tag };
            return JUMP_TABLE[tag as usize](self $(, $arg)*);
            static JUMP_TABLE: &'static [
                fn(&$EnumName $(, $arg_type)*) -> $ret
            ] = tagged_union_with_jump_tables! {
                @closures_table
                $EnumName $discriminant_type
                [ $($arg)* ]
                [ $( $Variant [ $( ($( $variant_field_type )*) )* ] )* ]
                [ $( $ReEnumName $MatchedVariant $( ($( $matched_field )*) )* $block )* ]
                []
            };
        }

        tagged_union_with_jump_tables! {
            @methods
            $discriminant_type
            $EnumName { $( $Variant $( ($( $variant_field_type )*) )* )* }
            $($tail)*
        }
    };
    (
        @methods
        $discriminant_type: ident
        $EnumName: ident { $( $Variant: ident $( ($( $variant_field_type: ty )*) )* )* }
    ) => {
        // Base case for recursion over methods
    };
    (
        @closures_table
        $EnumName: ident $discriminant_type: ident
        [ $($arg: ident)* ]
        [
            $Variant: ident
            [ $(
                ($( $variant_field_type: ty )*)
            )* ]
            $($variants_tail: tt)*
        ]
        [
            $ReEnumName: ident $MatchedVariant: ident
            $( ($( $matched_field: ident )*) )*
            $block: block
            $($matched_tail: tt)*
        ]
        [ $($previous_closures: tt)* ]
    ) => {{
        tagged_union_with_jump_tables! {
            @assert_idents_equal expected = $EnumName, found = $ReEnumName
        }
        tagged_union_with_jump_tables! {
            @assert_idents_equal expected = $Variant, found = $MatchedVariant
        }
        tagged_union_with_jump_tables! {
            @closures_table
            $EnumName $discriminant_type
            [ $($arg)* ]
            [ $( $variants_tail )* ]
            [ $( $matched_tail )* ]
            [
                $( $previous_closures )*

                |self_ $(, $arg)*| {
                    // Suppress unused_variable warnings
                    // because some arguments might only be used in some closures.
                    $(
                        let _ = &$arg;
                    )*
                    let ptr: *const $EnumName = self_;
                    #[repr(C)]
                    struct Variant(
                        $discriminant_type  // tag
                        $( $(, $variant_field_type )* )*
                    );
                    let ptr = ptr as *const Variant;
                    let &Variant(_, $( $( ref $matched_field ),* )*) = unsafe { &*ptr };
                    $block
                },
            ]
        }
    }};
    (
        @closures_table
        $EnumName: ident $discriminant_type: ident
        [ $($arg: ident)* ]
        [ ]
        [ ]
        [ $($closures: tt)* ]
    ) => {
        &[ $($closures)* ]
    };
    (@assert_ident_is_int u8) => {};
    (@assert_ident_is_int u16) => {};
    (@assert_ident_is_int u32) => {};
    (@assert_ident_is_int u64) => {};
    (@assert_ident_is_int i8) => {};
    (@assert_ident_is_int i16) => {};
    (@assert_ident_is_int i32) => {};
    (@assert_ident_is_int i64) => {};
    (@assert_idents_equal expected = $expected: ident, found = $found: ident) => {
        // When expanding tagged_union_with_jump_tables,
        // this expands to a macro that expects a single specific concrete ident,
        // and an invocation of that macro with another concrete ident.
        // For example:
        //
        //     macro_rules! assert_is_as_expected { (Foo) => {} }
        //     assert_ident!(Bar);
        macro_rules! assert_is_as_expected { ($expected) => {} }
        assert_is_as_expected!($found);
    };
}

#[test]
fn it_works() {
    tagged_union_with_jump_tables! {
        #[repr(u16)]
        enum Foo {
            V1(u8, String),
            V2(&'static str),
            V3,
        }

        fn get(&self, x: u8) -> (&str, u8) {
            match *self {
                Foo::V1(ref u, ref s) =>  { (&**s, *u) }
                Foo::V2(ref s) => { (s, x) }
                Foo::V3 => { ("3", x) }
            }
        }
    }
    assert_eq!(Foo::V1(1, "".into()).get(0), ("", 1));
    assert_eq!(Foo::V2("bar").get(5), ("bar", 5));
    assert_eq!(Foo::V3.get(10), ("3", 10));
}
