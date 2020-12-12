//! Macros of rsfbclient

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields};

/// Derive an [IntoParams<T>](../trait.IntoParams.html) implementation for structs.
///
/// This enables passing an instance of such a struct in places where named parameters
/// are expected, using the field labels to associate field values with parameter names.
///
/// The fields' types must implement the [IntoParam<T>](../trait.IntoParam.html) trait.
///
/// Note that `Option<T>` may be used as a field type to indicate a nullable parameter.
///
/// Providing an instance of the struct with value `None` for such a field corresponds to
/// passing a `null` value for that field.
#[proc_macro_derive(IntoNamedParams)]
pub fn into_params_derive(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let st_name = &input.ident;
    let st_fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };
    let st_fields_params = st_fields
        .iter()
        .map(|field| field.ident.as_ref().expect("Field name required"))
        .map(|field| {
            let field_str = field.to_string();
            quote! { #field_str.to_string(), self.#field.into_param() }
        });

    let st_impl = quote! {
        use rsfbclient::{IntoStmtArgs, IntoParam, NamedParams};
        use std::collections::HashMap;

        impl IntoNamedParams for #st_name {
            fn into_named_params(self) -> NamedParams {
                let mut params = HashMap::new();

                #(params.insert(#st_fields_params));*;

                NamedParams(params)
            }
        }
    };

    TokenStream::from(st_impl)
}

//todo: Tuple params
//#[proc_macro]
//pub fn make_answer(_item: TokenStream) -> TokenStream {
//    "fn answer() -> u32 { 42 }".parse().unwrap()
//}
//
//
/////
///// Generates FromRow implementations for various tuples
//macro_rules! impls_into_args {
//    ([$t: ident, $v: ident]) => {
//        impl_into_args!([$t, $v]);
//    };
//
//    ([$t: ident, $v: ident], $([$ts: ident, $vs: ident]),+ ) => {
//        impls_into_args!($([$ts, $vs]),+);
//
//        impl_into_args!([$t, $v], $([$ts, $vs]),+);
//    };
//}
//
//impls_into_args!(
//    [A, a],
//    [B, b],
//    [C, c],
//    [D, d],
//    [E, e],
//    [F, f],
//    [G, g],
//    [H, h],
//    [I, i],
//    [J, j],
//    [K, k],
//    [L, l],
//    [M, m],
//    [N, n],
//    [O, o]
//);
//
//macro_rules! impl_into_args {
//    ( $([$t: ident, $v: ident]),+ ) => {
//        impl<$($t),+> IntoStmtArgs for ($($t,)+)
//        where
//            $( $t: IntoStmtArg, )+
//        {
//            fn to_args(self) -> Vec<SqlType> {
//                let ( $($v,)+ ) = self;
//
//                vec![ $(
//                    $v.into_arg(),
//                )+ ]
//            }
//        }
//    };
//}
///// Generates FromRow implementations for a tuple
//macro_rules! impl_from_row {
//    ($($t: ident),+) => {
//        impl<'a, $($t),+> FromRow for ($($t,)+)
//        where
//            $( Column: ColumnToVal<$t>, )+
//        {
//            fn try_from(row: Vec<Column>) -> Result<Self, FbError> {
//                let len = row.len();
//                let mut iter = row.into_iter();
//
//                Ok(( $(
//                    ColumnToVal::<$t>::to_val(
//                        iter
//                            .next()
//                            .ok_or_else(|| {
//                                FbError::Other(
//                                    format!("The sql returned less columns than the {} expected", len),
//                                )
//                            })?
//                    )?,
//                )+ ))
//            }
//        }
//    };
//}
//
///// Generates FromRow implementations for various tuples
//macro_rules! impls_from_row {
//    ($t: ident) => {
//        impl_from_row!($t);
//    };
//
//    ($t: ident, $($ts: ident),+ ) => {
//        impls_from_row!($($ts),+);
//
//        impl_from_row!($t, $($ts),+);
//    };
//}
//
//impls_from_row!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);
