// Copyright 2022-2023 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Field, Fields, GenericArgument, Ident, PathArguments, Result, Type,
    TypePath,
};

/// Helper function to derive the trait that designates an empty enum as a list.
///
/// Example parameters for the doubly linked list:
/// * trait_name: "NtList"
/// * trait_path: quote! {::nt_list::list::traits::NtList}
pub(crate) fn derive_list_enum_trait(
    input: DeriveInput,
    list_type_name: &str,
    list_type_path: TokenStream,
) -> Result<TokenStream> {
    if let Data::Enum(e) = &input.data {
        if e.variants.is_empty() {
            let ident = &input.ident;

            return Ok(quote! {
                impl ::nt_list::NtTypedList for #ident {
                    type T = #list_type_path;
                }
            });
        }
    }

    Err(Error::new_spanned(
        input,
        format!("{} can only be derived for an empty enum", list_type_name),
    ))
}

/// Helper function to derive NtListElement.
pub fn derive_list_struct_trait(input: DeriveInput) -> Result<TokenStream> {
    let s = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return Err(Error::new_spanned(
                input,
                "NtListElement can only be derived for structs",
            ))
        }
    };

    let f = match &s.fields {
        Fields::Named(f) => f,
        _ => {
            return Err(Error::new_spanned(
                input,
                "NtListElement can only be derived for structs with named fields",
            ))
        }
    };

    if !has_repr_c(&input) {
        return Err(Error::new_spanned(
            input,
            "NtListElement can only be derived for structs with #[repr(C)]",
        ));
    }

    let mut boxed_attrs = 0usize;
    let ident = &input.ident;

    let tokens = f.named.iter().filter_map(|field| {
        parse_element_field(field).map(|info| {
            let field_ident = info.ident;
            let list_ty = info.list_ty;
            boxed_attrs += info.is_boxed as usize;

            let mut boxed_impl = TokenStream::new();
            if info.is_boxed {
                boxed_impl = quote! {
                    impl ::nt_list::NtBoxedListElement for #ident {
                        type L = #list_ty;
                    }
                };
            }

            quote! {
                unsafe impl ::nt_list::NtListElement<#list_ty> for #ident {
                    fn offset() -> usize {
                        let base = ::core::mem::MaybeUninit::<#ident>::uninit();
                        let base_ptr = base.as_ptr();
                        let field_ptr = unsafe { ::core::ptr::addr_of!((*base_ptr).#field_ident) };
                        field_ptr as usize - base_ptr as usize
                    }
                }

                #boxed_impl
            }
        })
    });
    let output = quote! {
        #(#tokens)*
    };

    if output.is_empty() {
        return Err(Error::new_spanned(
            input,
            "Found no NtListEntry/NtSingleListEntry fields",
        ));
    }

    if boxed_attrs > 1 {
        return Err(Error::new_spanned(
            input,
            "Only a single entry field may have a #[boxed] attribute",
        ));
    }

    Ok(output)
}

/// Returns whether the given input has a `#[repr(C)]` attribute.
///
/// This also works when multiple `repr` attributes are used, or a single `repr` attribute has multiple entries.
fn has_repr_c(input: &DeriveInput) -> bool {
    let mut repr_c = false;

    for attr in &input.attrs {
        if attr.path().is_ident("repr") {
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("C") {
                    repr_c = true;
                }

                Ok(())
            });
        }
    }

    repr_c
}

pub(crate) struct ElementFieldInfo<'a> {
    /// The "entry" in `entry: nt_list::list::base::NtListEntry<Self, mytraits::MyList>`
    pub(crate) ident: &'a Ident,
    /// The "mytraits::MyList" in `entry: nt_list::list::base::NtListEntry<Self, mytraits::MyList>`
    pub(crate) list_ty: &'a TypePath,
    /// Whether a `#[boxed]` attribute has been placed before the field.
    pub(crate) is_boxed: bool,
}

/// Checks if the given field is a list entry field of an element structure and returns some
/// information about it.
///
/// `field` can be the syntax tree of e.g.
/// * `entry: NtListEntry<Self, MyList>`
/// * `entry: nt_list::list::base::NtListEntry<Self, mytraits::MyList>`
pub(crate) fn parse_element_field(field: &Field) -> Option<ElementFieldInfo> {
    const SUPPORTED_TYPES: &[&str] = &["NtListEntry", "NtSingleListEntry"];

    let ident = &field.ident.as_ref()?;
    let is_boxed = field.attrs.iter().any(|attr| attr.path().is_ident("boxed"));

    // Get the last segment of the type path and check it against the type name.
    // This isn't 100% accurate, we may catch similarly named types that are not ours.
    // But a user who derives `NtListElement` for a structure shouldn't mix it with foreign `NtListEntry` types anyway...
    let ty_path = match &field.ty {
        Type::Path(ty_path) => ty_path,
        _ => return None,
    };

    let segment = ty_path.path.segments.last()?;
    if !SUPPORTED_TYPES.iter().any(|x| segment.ident == x) {
        return None;
    }

    // Make our check more accurate by also checking that the `NtListEntry` type of this field has two type parameters.
    let ab_args = match &segment.arguments {
        PathArguments::AngleBracketed(ab_args) => ab_args,
        _ => return None,
    };
    if ab_args.args.len() != 2 {
        return None;
    }

    // Now we can be reasonably sure that this is our `NtListEntry` type and the second type parameter is the one
    // we are looking for.
    let arg = ab_args.args.last()?;
    let ty = match &arg {
        GenericArgument::Type(ty) => ty,
        _ => return None,
    };
    let list_ty = match &ty {
        Type::Path(list_ty) => list_ty,
        _ => return None,
    };

    Some(ElementFieldInfo {
        ident,
        list_ty,
        is_boxed,
    })
}
