// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

mod helpers;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(NtList)]
pub fn derive_nt_list(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    helpers::derive_list_enum_trait(input, "NtList", quote! {::nt_list::list::traits::NtList})
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(NtListElement, attributes(boxed))]
pub fn derive_nt_list_element(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    helpers::derive_list_struct_trait(
        input,
        "NtListElement",
        quote! {::nt_list::list::traits::NtListElement},
        "NtListEntry",
        quote! {::nt_list::list::traits::NtBoxedListElement},
    )
    .unwrap_or_else(|e| e.to_compile_error())
    .into()
}
