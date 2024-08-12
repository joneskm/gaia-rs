#![cfg(not(doctest))]
#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/","Readme.md"))]

use darling::FromAttributes;
use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput};

mod params_key;
mod store_key;

#[proc_macro_derive(ParamsKeys, attributes(pkey))]
pub fn params_derive(input: TokenStream) -> TokenStream {
    params_key::expand_params(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(StoreKeys, attributes(skey))]
pub fn store_derive(input: TokenStream) -> TokenStream {
    store_key::expand_store(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[derive(FromAttributes, Default)]
#[darling(default, attributes(pkey, skey), forward_attrs(allow, doc, cfg))]
#[darling(and_then = Self::not_empty)]
struct KeysAttr {
    pub to_string: String,
}

impl KeysAttr {
    fn not_empty(self) -> darling::Result<Self> {
        if self.to_string.is_empty() {
            Err(darling::Error::custom("key can't be empty").with_span(&self.to_string.span()))
        } else {
            Ok(self)
        }
    }
}
