use std::collections::HashSet;

use darling::{util::Flag, FromAttributes, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, DataEnum, DeriveInput, Ident, Variant};

#[derive(FromDeriveInput)]
#[darling(attributes(skey))]
struct KeysArg {
    #[darling(default)]
    pub gears: Flag,
    pub params: Ident,
}

#[derive(FromAttributes, Default)]
#[darling(default, attributes(skey), forward_attrs(allow, doc, cfg))]
#[darling(and_then = Self::not_empty)]
struct KeysAttr {
    pub to_string: String,
}

impl KeysAttr {
    fn not_empty(self) -> darling::Result<Self> {
        if self.to_string.is_empty() || self.to_string.replace(' ', "").is_empty() {
            Err(darling::Error::custom("key can't be empty").with_span(&self.to_string.span()))
        } else {
            Ok(self)
        }
    }
}

pub fn expand_store(input: DeriveInput) -> syn::Result<TokenStream> {
    let KeysArg { gears, params } = KeysArg::from_derive_input(&input)?;
    let DeriveInput { ident, data, .. } = input;

    match data {
        syn::Data::Enum(DataEnum { variants, .. }) => {
            let crate_prefix = match gears.is_present() {
                true => quote! { crate },
                false => quote! { ::gears },
            };

            let mut enum_variants = Vec::<TokenStream>::new();
            let mut set = HashSet::<String>::with_capacity(enum_variants.len());

            for Variant { attrs, ident, .. } in variants {
                let KeysAttr { to_string } = KeysAttr::from_attributes(&attrs)?;

                if let Some(prefix) =
                    set.iter()
                        .find(|this| match this.len().cmp(&to_string.len()) {
                            std::cmp::Ordering::Less => to_string.starts_with(*this),
                            std::cmp::Ordering::Equal => this == &&to_string,
                            std::cmp::Ordering::Greater => this.starts_with(&to_string),
                        })
                {
                    Err(syn::Error::new(
                        ident.span(),
                        format!("Key: {to_string} is prefix of another item: {prefix}"),
                    ))?
                }

                let _ = set.insert(to_string.clone());

                enum_variants.push(quote! { Self::#ident => #to_string });
            }

            let result = quote! {
                impl #crate_prefix ::store::StoreKey for #ident
                {
                    fn name(&self) -> &'static str
                    {
                        match self {
                            #(#enum_variants),*
                        }
                    }

                    fn params() -> &'static Self {
                        const PARAM_KEY: #ident = #ident::#params;

                        &PARAM_KEY
                    }
                }
            };

            Ok(result)
        }
        _ => Err(syn::Error::new(
            ident.span(),
            "Can be derived only on enums",
        )),
    }
}
