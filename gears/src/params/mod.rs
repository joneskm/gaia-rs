pub mod gas;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    str::FromStr,
};

use database::{prefix::PrefixDB, Database};
use extensions::corruption::UnwrapCorrupt;
use kv_store::StoreKey;

use crate::context::{InfallibleContext, InfallibleContextMut};

use self::{parsed::Params, space::ParamsSpace, space_mut::ParamsSpaceMut};

pub mod parsed;
pub mod space;
pub mod space_mut;

pub fn infallible_subspace<
    'a,
    DB: Database,
    SK: StoreKey,
    CTX: InfallibleContext<DB, SK>,
    PSK: ParamsSubspaceKey,
>(
    ctx: &'a CTX,
    params_subspace_key: &PSK,
) -> ParamsSpace<'a, PrefixDB<DB>> {
    ParamsSpace {
        inner: InfallibleContext::infallible_store(ctx, SK::params())
            .prefix_store(params_subspace_key.name().as_bytes().to_vec()),
    }
}

pub fn infallible_subspace_mut<
    'a,
    DB: Database,
    SK: StoreKey,
    CTX: InfallibleContextMut<DB, SK>,
    PSK: ParamsSubspaceKey,
>(
    ctx: &'a mut CTX,
    params_subspace_key: &PSK,
) -> ParamsSpaceMut<'a, PrefixDB<DB>> {
    ParamsSpaceMut {
        inner: InfallibleContextMut::infallible_store_mut(ctx, SK::params())
            .prefix_store_mut(params_subspace_key.name().as_bytes().to_vec()),
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
#[error("error parsing subpsace: {0}")]
pub struct SubspaceParseError(pub String);

pub trait ParamsSubspaceKey: std::fmt::Debug + Hash + Eq + Clone + Send + Sync + 'static {
    fn name(&self) -> String;

    fn from_subspace_str(val: impl AsRef<str>) -> Result<Self, SubspaceParseError>;
}

pub trait ParamsSerialize {
    /// Return all unique keys for this structure
    fn keys() -> HashSet<&'static str>;
    fn to_raw(&self) -> Vec<(&'static str, Vec<u8>)>;
}

pub trait ParamsDeserialize: ParamsSerialize {
    fn from_raw(fields: HashMap<&'static str, Vec<u8>>) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParamKind {
    Bytes,
    String,
    Bool,
    U64,
    I64,
    U32,
    I32,
    U16,
    I16,
    U8,
    I8,
}

impl ParamKind {
    pub fn parse_param(self, bytes: Vec<u8>) -> Params {
        fn parse_primitive_bytes<T: FromStr>(value: Vec<u8>) -> T
        where
            <T as FromStr>::Err: std::fmt::Debug,
        {
            let value = String::from_utf8(value).expect("should be valid utf-8");

            value
                .strip_suffix('\"')
                .and_then(|this| this.strip_prefix('\"'))
                .map(String::from)
                .unwrap_or(value)
                .parse()
                .unwrap_or_corrupt()
        }

        match self {
            ParamKind::Bytes => Params::Bytes(bytes),
            ParamKind::String => match String::from_utf8(bytes) {
                Ok(var) => Params::String(var),
                Err(err) => Params::InvalidCast(err.into_bytes()),
            },
            ParamKind::Bool => match bool::from_str(&String::from_utf8_lossy(&bytes)) {
                Ok(var) => Params::Bool(var),
                Err(_) => Params::InvalidCast(bytes),
            },
            ParamKind::U64 => Params::U64(parse_primitive_bytes(bytes)),
            ParamKind::I64 => Params::I64(parse_primitive_bytes(bytes)),
            ParamKind::U32 => Params::U32(parse_primitive_bytes(bytes)),
            ParamKind::I32 => Params::I32(parse_primitive_bytes(bytes)),
            ParamKind::U16 => Params::U16(parse_primitive_bytes(bytes)),
            ParamKind::I16 => Params::I16(parse_primitive_bytes(bytes)),
            ParamKind::U8 => Params::U8(parse_primitive_bytes(bytes)),
            ParamKind::I8 => Params::I8(parse_primitive_bytes(bytes)),
        }
    }
}
