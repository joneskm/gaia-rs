use darling::{util::Flag, FromAttributes, FromDeriveInput};
use quote::quote;
use syn::{spanned::Spanned, DataStruct, DeriveInput};

use crate::{FieldWrapper, OptionalOrRepeated};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(proto))]
struct ProtobufArg {
    #[darling(default)]
    raw: Option<syn::TypePath>,
    #[darling(default)]
    gears: Flag,
}

#[derive(FromAttributes, Default)]
#[darling(default, attributes(proto), forward_attrs(allow, doc, cfg))]
struct ProtobufAttr {
    name: Option<syn::Ident>,
    #[darling(flatten, default)]
    opt: OptionalOrRepeated,
    from: Option<syn::TypePath>,
    from_ref: Flag,
    into: Option<syn::TypePath>,
    into_ref: Flag,
}

pub fn expand_raw_existing(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ProtobufArg { raw, gears } = ProtobufArg::from_derive_input(&input)?;
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = input;

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let crate_prefix = match gears.is_present() {
        true => quote! { crate },
        false => quote! { ::gears },
    };

    let raw = match raw {
        Some(raw) => quote! { #raw },
        None => {
            let new_name = syn::Ident::new(&format!("Raw{ident}"), proc_macro2::Span::call_site());

            quote! { #new_name }
        }
    };

    let protobuf_trait_impl = quote! {
        impl #impl_generics #crate_prefix ::core::Protobuf<#raw> for #ident #ty_generics #where_clause  {}
    };

    match data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let mut raw_fields = Vec::new();
            for field in fields {
                let ProtobufAttr {
                    name,
                    opt,
                    from,
                    into,
                    from_ref,
                    into_ref,
                } = ProtobufAttr::from_attributes(&field.attrs)?;

                let field_indent = field.ident.clone().ok_or(syn::Error::new(
                    field.span(),
                    "Can't derive on tuple structures",
                ))?;

                let name = name.clone().unwrap_or(field_indent.clone());

                raw_fields.push((
                    name,
                    field_indent,
                    FieldWrapper::from_type(&field.ty)?,
                    opt.kind(),
                    (from, from_ref),
                    (into, into_ref),
                ));
            }

            let from_fields_iter_gen = {
                let mut from_fields = Vec::with_capacity(raw_fields.len());

                for (
                    other_name,
                    field_ident,
                    field_kind,
                    other_field_kind,
                    _from,
                    (into, into_ref),
                ) in &raw_fields
                {
                    let into_method = match into.as_ref() {
                        Some(var) => quote! { #var },
                        None => quote! { ::std::convert::Into::into },
                    };

                    let value_prefix = match into_ref.is_present() {
                        true => quote! { & },
                        false => quote! {},
                    };

                    let result = match (field_kind, other_field_kind) {
                        (FieldWrapper::Optional, FieldWrapper::Optional) => quote! {
                            #other_name : match value.#field_ident
                            {
                                ::std::option::Option::Some(var) => ::std::option::Option::Some( #into_method ( #value_prefix var)),
                                ::std::option::Option::None => ::std::option::Option::None,
                            }
                        },
                        (FieldWrapper::Optional, FieldWrapper::Vec) => {
                            Err(syn::Error::new_spanned(field_ident, "Can't cast Option"))?
                        }
                        (FieldWrapper::Optional, FieldWrapper::None) => {
                            Err(syn::Error::new_spanned(
                                field_ident,
                                "Can't have optional while raw is required",
                            ))?
                        }
                        (FieldWrapper::Vec, FieldWrapper::Optional) => Err(
                            syn::Error::new_spanned(field_ident, "Can't cast Vec to Option"),
                        )?,
                        (FieldWrapper::Vec, FieldWrapper::Vec) => quote! {
                            #other_name : {
                                let mut buffer = std::vec::Vec::with_capacity(value.#field_ident.len());

                                for field in value.#field_ident
                                {
                                    buffer.push( #into_method ( #value_prefix field) );
                                }

                                buffer
                            }
                        },
                        (FieldWrapper::Vec, FieldWrapper::None) => Err(syn::Error::new_spanned(
                            field_ident,
                            "Can't cast Vec to field",
                        ))?,
                        (FieldWrapper::None, FieldWrapper::Optional) => quote! {
                            #other_name : ::std::option::Option::Some( #into_method ( #value_prefix value.#field_ident))
                        },
                        (FieldWrapper::None, FieldWrapper::Vec) => Err(syn::Error::new_spanned(
                            field_ident,
                            "Can't cast Vec to field",
                        ))?,
                        (FieldWrapper::None, FieldWrapper::None) => quote! {
                            #other_name : #into_method (#value_prefix value.#field_ident)
                        },
                    };

                    from_fields.push(result);
                }

                from_fields
            };

            let from_impl = quote! {
                impl #impl_generics ::std::convert::From<#ident #ty_generics > for #raw #where_clause {
                    fn from(value: #ident #ty_generics) -> Self {
                        Self
                        {
                            #(#from_fields_iter_gen),*
                        }
                    }
                }
            };

            let try_from_fields_iter_gen = {
                let mut from_fields = Vec::with_capacity(raw_fields.len());

                for (
                    other_name,
                    field_ident,
                    field_kind,
                    other_field_kind,
                    (from, from_ref),
                    _into,
                ) in raw_fields
                {
                    let from_method = match from.as_ref() {
                        Some(var) => quote! { #var },
                        None => quote! { ::std::convert::TryFrom::try_from },
                    };

                    let value_prefix = match from_ref.is_present() {
                        true => quote! { & },
                        false => quote! {},
                    };

                    let result = match (field_kind, other_field_kind) {
                        (FieldWrapper::Optional, FieldWrapper::Optional) => quote! {
                            #field_ident : match value.#other_name {
                                Some(var) => Some( #from_method (#value_prefix var)?),
                                None => None,
                            }
                        },
                        (FieldWrapper::Optional, FieldWrapper::Vec) => Err(
                            syn::Error::new_spanned(field_ident, "Can't cast Vec to Option"),
                        )?,
                        (FieldWrapper::Optional, FieldWrapper::None) => quote! {
                            #field_ident : ::std::option::Option::Some( #from_method (#value_prefix value.#other_name))
                        },
                        (FieldWrapper::Vec, FieldWrapper::Optional) => Err(
                            syn::Error::new_spanned(field_ident, "Can't cast Vec to Option"),
                        )?,
                        (FieldWrapper::Vec, FieldWrapper::Vec) => quote! {
                            #field_ident : {
                                let mut buffer = std::vec::Vec::with_capacity(#value_prefix value.#other_name.len());

                                for field in value.#other_name
                                {
                                    buffer.push( #from_method (field)?);
                                }

                                buffer
                            }
                        },
                        (FieldWrapper::Vec, FieldWrapper::None) => Err(syn::Error::new_spanned(
                            field_ident,
                            "Can't cast Vec to field",
                        ))?,
                        (FieldWrapper::None, FieldWrapper::Optional) => {
                            let other_name_str = other_name.to_string();

                            quote! {
                                #field_ident : match value.#other_name
                                {
                                    ::std::option::Option::Some(var) => ::std::result::Result::Ok( #from_method (#value_prefix var)?),
                                    ::std::option::Option::None => ::std::result::Result::Err( #crate_prefix ::error::ProtobufError::MissingField( ::std::format!( "Missing field: {}", #other_name_str ))),
                                }?
                            }
                        }
                        (FieldWrapper::None, FieldWrapper::Vec) => Err(syn::Error::new_spanned(
                            field_ident,
                            "Can't cast Vec to field",
                        ))?,
                        (FieldWrapper::None, FieldWrapper::None) => quote! {
                            #field_ident : #from_method (#value_prefix value.#other_name)?
                        },
                    };

                    from_fields.push(result);
                }

                from_fields
            };

            let try_from = quote! {

                impl #impl_generics TryFrom<#raw> for #ident #ty_generics #where_clause  {
                    type Error = #crate_prefix ::error::ProtobufError;

                    fn try_from(value: #raw) -> ::std::result::Result<Self, Self::Error> {
                        ::std::result::Result::Ok(Self {
                            #(#try_from_fields_iter_gen),*
                        })
                    }
                }

            };

            let gen = quote! {
                #try_from

                #from_impl

                #protobuf_trait_impl
            };

            Ok(gen)
        }
        _ => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Protobuf can be derived only for `struct`",
        )),
    }
}
