// Copyright 2022 Colin Finck <colin@reactos.org>
// SPDX-License-Identifier: MIT OR Apache-2.0

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Error, Field, Fields, GenericArgument, Ident, PathArguments, Result, Type,
    TypePath,
};

/// Helper function to derive the trait that designates an empty enum as a list.
///
/// Example parameters for the doubly-linked list:
/// * trait_name: "NtList"
/// * trait_path: quote! {crate::list::traits::NtList}
pub(crate) fn derive_list_enum_trait(
    input: DeriveInput,
    trait_name: &str,
    trait_path: TokenStream,
) -> Result<TokenStream> {
    if let Data::Enum(e) = &input.data {
        if e.variants.is_empty() {
            let ident = &input.ident;

            return Ok(quote! {
                impl #trait_path for #ident {}
            });
        }
    }

    Err(Error::new_spanned(
        input,
        format!("{} can only be derived for an empty enum", trait_name),
    ))
}

/// Helper function to derive the trait that designates a structure as a list element.
///
/// Example parameters for the doubly-linked list:
/// * trait_name: "NtListElement"
/// * trait_path: quote! {crate::list::traits::NtListElement}
/// * field_ty_name: "NtListEntry"
/// * boxed_trait_path: quote! {crate::list::traits::NtBoxedListElement}
pub fn derive_list_struct_trait(
    input: DeriveInput,
    trait_name: &str,
    trait_path: TokenStream,
    field_ty_name: &str,
    boxed_trait_path: TokenStream,
) -> Result<TokenStream> {
    let s = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return Err(Error::new_spanned(
                input,
                format!("{} can only be derived for structs", trait_name),
            ))
        }
    };

    let f = match &s.fields {
        Fields::Named(f) => f,
        _ => {
            return Err(Error::new_spanned(
                input,
                format!(
                    "{} can only be derived for structs with named fields",
                    trait_name
                ),
            ))
        }
    };

    let mut boxed_attrs = 0usize;
    let ident = &input.ident;

    let tokens = f.named.iter().filter_map(|field| {
        parse_element_field(field, field_ty_name).map(|info| {
            let field_ident = info.ident;
            let list_ty = info.list_ty;
            boxed_attrs += info.is_boxed as usize;

            let mut boxed_impl = TokenStream::new();
            if info.is_boxed {
                boxed_impl = quote! {
                    impl #boxed_trait_path for #ident {
                        type L = #list_ty;
                    }
                };
            }

            quote! {
                impl #trait_path<#list_ty> for #ident {
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
            format!("Found no {} fields", field_ty_name),
        ));
    }

    if boxed_attrs > 1 {
        return Err(Error::new_spanned(
            input,
            format!(
                "Only a single {} field may have a #[boxed] attribute",
                field_ty_name
            ),
        ));
    }

    Ok(output)
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
pub(crate) fn parse_element_field<'a>(
    field: &'a Field,
    ty_name: &str,
) -> Option<ElementFieldInfo<'a>> {
    let ident = &field.ident.as_ref()?;
    let is_boxed = field
        .attrs
        .iter()
        .find(|attr| attr.path.is_ident("boxed"))
        .is_some();

    // Get the last segment of the type path and check it against the type name.
    // This isn't 100% accurate, we may catch similarly named types that are not ours.
    // But a user who derives `NtListElement` for a structure shouldn't mix it with foreign `NtListEntry` types anyway...
    let ty_path = match &field.ty {
        Type::Path(ty_path) => ty_path,
        _ => return None,
    };

    let segment = ty_path.path.segments.last()?;
    if segment.ident != ty_name {
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