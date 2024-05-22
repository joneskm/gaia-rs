use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use database::{prefix::PrefixDB, Database};
use store_crate::{QueryableKVStore, StoreKey, TransactionalKVStore};

use crate::types::context::{QueryableContext, TransactionalContext};

use self::{space::ParamsSpace, space_mut::ParamsSpaceMut, string::ParamString};

pub mod space;
pub mod space_mut;
pub mod string;

pub fn subspace<
    'a,
    DB: Database,
    SK: StoreKey,
    CTX: QueryableContext<DB, SK>,
    PSK: ParamsSubspaceKey,
>(
    ctx: &'a CTX,
    store_key: &SK,
    params_subspace_key: &PSK,
) -> ParamsSpace<'a, PrefixDB<DB>> {
    ParamsSpace {
        inner: ctx
            .kv_store(store_key)
            .prefix_store(params_subspace_key.name().as_bytes().to_vec()),
    }
}

pub fn subspace_mut<
    'a,
    DB: Database,
    SK: StoreKey,
    CTX: TransactionalContext<DB, SK>,
    PSK: ParamsSubspaceKey,
>(
    ctx: &'a mut CTX,
    store_key: &SK,
    params_subspace_key: &PSK,
) -> ParamsSpaceMut<'a, PrefixDB<DB>> {
    ParamsSpaceMut {
        inner: ctx
            .kv_store_mut(store_key)
            .prefix_store_mut(params_subspace_key.name().as_bytes().to_vec()),
    }
}

pub trait ParamsSubspaceKey: Hash + Eq + Clone + Send + Sync + 'static {
    fn name(&self) -> &'static str;
}

// TODO:LATER For PR with xmod to change any params
// pub trait ModuleParams {
//     fn module_params<PSK: ParamsSubspaceKey, P: Params>() -> (PSK, P);
// }

pub trait Params {
    /// Return all unique keys for this structure
    fn keys() -> HashSet<&'static str>;
    fn to_raw(&self) -> HashMap<&'static str, ParamString>;
}

pub trait ParamsDeserialize: Params {
    fn from_raw(fields: HashMap<&'static str, ParamString>) -> Self;
}

/// Parse params bytes into valid `String` which must we able to parse into param ***field***
fn parse_param(value: String) -> ParamString {
    // Some types like `bool` gets saved without
    if let Some(cleared) = value
        .strip_suffix('\"')
        .and_then(|this| this.strip_prefix('\"'))
    {
        cleared.into()
    } else {
        value.into()
    }
}

pub fn parse_primitive<T: From<ParamString>>(value: Option<ParamString>) -> Option<T> {
    match value {
        Some(value) => Some(parse_param(value.0).into()),
        None => None,
    }
}

pub fn parse_primitive_unwrap<T: From<ParamString>>(value: Option<ParamString>) -> T {
    parse_param(value.expect("Params expected to exists").0).into()
}
