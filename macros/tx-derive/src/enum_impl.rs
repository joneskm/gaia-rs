use darling::FromAttributes;
use quote::quote;
use syn::{DataEnum, Generics, Ident};

use crate::MessageAttr;

pub fn expand_macro(
    DataEnum { variants, .. }: DataEnum,
    type_ident: Ident,
    generics: Generics,
    crate_prefix: proc_macro2::TokenStream,
) -> syn::Result<proc_macro2::TokenStream> {
    let get_signers = variants.iter().map(|v| v.clone().ident).map(|i| {
        quote! {
            Self::#i(msg) => #crate_prefix::types::tx::TxMessage::get_signers(msg)
        }
    });

    let type_url = variants.iter().map(|v| v.clone().ident).map(|i| {
        quote! {
            Self::#i(msg) => #crate_prefix::types::tx::TxMessage::type_url(msg)
        }
    });

    let amino_url = variants.iter().map(|v| v.clone().ident).map(|i| {
        quote! {
            Self::#i(msg) => #crate_prefix::types::tx::TxMessage::amino_url(msg)
        }
    });

    let into_any = variants
        .iter()
        .map(|v: &syn::Variant| v.clone().ident)
        .map(|i| {
            quote! {
                #type_ident ::#i(msg) => msg.into()
            }
        });

    let mut from_any = Vec::new();
    for v in &variants {
        let attr = &v.attrs;
        let ident = &v.ident;

        let MessageAttr { url, signer: _ } = MessageAttr::from_attributes(attr)?;
        let url = match url {
            Some(url) => quote! { #url },
            None => Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "`url` attribute is required for enum variant",
            ))?,
        };

        from_any.push(quote! {
            if value.type_url.starts_with(#url) {
                Ok(Self::#ident(#crate_prefix::core::any::google::Any::try_into(value)?))
            }
        });
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let gen = quote! {
        impl #impl_generics  #crate_prefix::types::tx::TxMessage for #type_ident #ty_generics #where_clause {

            fn get_signers(&self) -> Vec<&#crate_prefix::types::address::AccAddress> {

                match self {
                    #(#get_signers),*
                }
            }

            fn type_url(&self) -> &'static str {
                match self {
                    #(#type_url),*
                }
            }

            fn amino_url(&self) -> &'static str {
                match self {
                    #(#amino_url),*
                }
            }

        }

        impl #impl_generics From<#type_ident #ty_generics> for #crate_prefix::core::any::google::Any #where_clause {
            fn from(msg: #type_ident #ty_generics) -> Self {
                match msg {
                    #(#into_any),*
                }
            }
        }

        impl #impl_generics TryFrom<#crate_prefix::core::any::google::Any> for #type_ident #ty_generics #where_clause {
            type Error = #crate_prefix::core::errors::CoreError;

            fn try_from(value: #crate_prefix::core::any::google::Any) -> Result<Self, Self::Error> {

                #(#from_any) else*

                 else {
                    Err(#crate_prefix::core::errors::CoreError::DecodeGeneral(
                        "message type not recognized".into(),
                    ))
                }
            }
        }

    };

    Ok(gen)
}
