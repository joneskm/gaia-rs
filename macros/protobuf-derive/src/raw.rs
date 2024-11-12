use darling::{util::PathList, FromAttributes, FromDeriveInput, FromMeta};
use quote::quote;
use syn::{DataStruct, DeriveInput, Field, TypePath};

use crate::OptionalOrRepeated;

#[derive(FromMeta)]
#[darling(rename_all = "lowercase")]
enum Kind {
    Sint32,
    Int64,
    Uint32,
    Uint64,
    Bool,
    String,
    Bytes,
    Message,
}

impl Kind {
    fn to_prost_token(&self) -> proc_macro2::TokenStream {
        match self {
            Kind::Sint32 => quote! { int32 },
            Kind::Int64 => quote! { int64 },
            Kind::Uint32 => quote! { uint32 },
            Kind::Uint64 => quote! { uint64 },
            Kind::Bool => quote! { r#bool },
            Kind::String => quote! { string },
            Kind::Bytes => quote! { bytes },
            Kind::Message => quote! { message },
        }
    }
}

#[derive(FromAttributes)]
#[darling(attributes(raw))]
struct RawAttr {
    #[darling(default)]
    raw: Option<syn::Path>,
    #[darling(flatten, default)]
    opt: OptionalOrRepeated,
    kind: Kind,
    #[darling(default)]
    tag: Option<u32>,
}

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(raw))]
struct RawArg {
    #[darling(default)]
    derive: PathList,
}

pub fn extend_new_structure(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let RawArg {
        derive: raw_derives,
    } = RawArg::from_derive_input(&input)?;
    let DeriveInput {
        vis, ident, data, ..
    } = input;

    match data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let mut result_fields = Vec::with_capacity(fields.len());
            let mut counter = 1;

            for Field {
                attrs,
                vis,
                ident,
                ty,
                ..
            } in fields
            {
                let RawAttr {
                    raw,
                    opt: OptionalOrRepeated { optional, repeated },
                    kind,
                    tag,
                } = RawAttr::from_attributes(&attrs)?;

                let raw = raw
                    .map(|path| syn::Type::Path(TypePath { qself: None, path }))
                    .unwrap_or(ty.clone());
                let tag = tag.unwrap_or(counter);

                let kind = kind.to_prost_token();

                let result = match (optional.is_present(), repeated.is_present()) {
                    (true, true) => unreachable!("we validated structure to omit such case"),
                    (true, false) => quote! {
                        #[prost( #kind, optional, tag = #tag )]
                        #vis #ident : ::std::option::Option<#raw>
                    },
                    (false, true) => quote! {
                       #[prost( #kind, repeated, tag = #tag )]
                       #vis #ident : ::prost::alloc::vec::Vec<#raw>
                    },
                    (false, false) => quote! {
                        #[prost( #kind, required, tag = #tag )]
                        #vis #ident : #raw
                    },
                };

                result_fields.push(result);

                counter = tag;
                counter += 1;
            }

            let new_name = syn::Ident::new(&format!("Raw{ident}"), proc_macro2::Span::call_site());

            let raw_derives = match raw_derives.is_empty() {
                true => quote! {},
                false => quote! { #[derive(#(#raw_derives,)*)] },
            };

            let gen = quote! {

                #[derive(::prost::Message)]
                #raw_derives
                #vis struct  #new_name
                {
                    #(#result_fields),*
                }
            };

            Ok(gen)
        }
        _ => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Protobuf can be derived only for `struct`",
        )),
    }
}
