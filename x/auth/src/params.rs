use database::Database;
use gears::types::context::context::Context;
use gears::x::{auth::Params, params::ParamsSubspaceKey};
use store::kv_store_key::{KvStoreKey, SimpleKvStoreKey};
//use params_module::ParamsSubspaceKey;
// use proto_messages::utils::serialize_number_to_string;
// use serde::{Deserialize, Serialize};
// use serde_aux::prelude::deserialize_number_from_string;
use store::{ImmutablePrefixStore, StoreKey};

// #[derive(Debug, Clone, Deserialize, Serialize)]
// pub struct Params {
//     #[serde(serialize_with = "serialize_number_to_string")]
//     #[serde(deserialize_with = "deserialize_number_from_string")]
//     pub max_memo_characters: u64,
//     #[serde(serialize_with = "serialize_number_to_string")]
//     #[serde(deserialize_with = "deserialize_number_from_string")]
//     pub tx_sig_limit: u64,
//     #[serde(serialize_with = "serialize_number_to_string")]
//     #[serde(deserialize_with = "deserialize_number_from_string")]
//     pub tx_size_cost_per_byte: u64,
//     #[serde(serialize_with = "serialize_number_to_string")]
//     #[serde(deserialize_with = "deserialize_number_from_string")]
//     pub sig_verify_cost_ed25519: u64,
//     #[serde(serialize_with = "serialize_number_to_string")]
//     #[serde(deserialize_with = "deserialize_number_from_string")]
//     pub sig_verify_cost_secp256k1: u64,
// }

const KEY_MAX_MEMO_CHARACTERS: [u8; 17] = [
    077, 097, 120, 077, 101, 109, 111, 067, 104, 097, 114, 097, 099, 116, 101, 114, 115,
]; // "MaxMemoCharacters"
const KEY_TX_SIG_LIMIT: [u8; 10] = [084, 120, 083, 105, 103, 076, 105, 109, 105, 116]; // "TxSigLimit"
const KEY_TX_SIZE_COST_PER_BYTE: [u8; 17] = [
    084, 120, 083, 105, 122, 101, 067, 111, 115, 116, 080, 101, 114, 066, 121, 116, 101,
]; // "TxSizeCostPerByte"
const KEY_SIG_VERIFY_COST_ED25519: [u8; 20] = [
    083, 105, 103, 086, 101, 114, 105, 102, 121, 067, 111, 115, 116, 069, 068, 050, 053, 053, 049,
    057,
]; // "SigVerifyCostED25519"
const KEY_SIG_VERIFY_COST_SECP256K1: [u8; 22] = [
    083, 105, 103, 086, 101, 114, 105, 102, 121, 067, 111, 115, 116, 083, 101, 099, 112, 050, 053,
    054, 107, 049,
]; // "SigVerifyCostSecp256k1"

pub const DEFAULT_PARAMS: Params = Params {
    max_memo_characters: 256,
    tx_sig_limit: 7,
    tx_size_cost_per_byte: 10,
    sig_verify_cost_ed25519: 590,
    sig_verify_cost_secp256k1: 1000,
};

#[derive(Debug, Clone)]
pub struct AuthParamsKeeper<SK: StoreKey, PSK: ParamsSubspaceKey> {
    pub params_keeper: gears::x::params::Keeper<SK, PSK>,
    pub params_subspace_key: PSK,
}

// TODO: add a macro to create this?
impl<SK: StoreKey, PSK: ParamsSubspaceKey> AuthParamsKeeper<SK, PSK> {
    fn parse_param(value: Vec<u8>) -> u64 {
        String::from_utf8(value)
            .expect("should be valid utf-8")
            .strip_suffix('\"')
            .expect("should have suffix")
            .strip_prefix('\"')
            .expect("should have prefix")
            .parse()
            .expect("should be valid u64")
    }

    fn get_raw_param<DB: Database>(
        key: impl KvStoreKey,
        store: &ImmutablePrefixStore<'_, DB>,
    ) -> Vec<u8> {
        store
            .get(key)
            .expect("key should be set in kv store")
            .clone()
    }

    pub fn get<T: Database>(&self, ctx: &Context<'_, '_, T, SK>) -> Params {
        let store = self
            .params_keeper
            .get_raw_subspace(ctx, &self.params_subspace_key);

        let raw = Self::get_raw_param(
            SimpleKvStoreKey::try_from(KEY_MAX_MEMO_CHARACTERS.as_ref())
                .expect("Unreachable. Const value should be valid"),
            &store,
        );
        let max_memo_characters = Self::parse_param(raw);

        let raw = Self::get_raw_param(
            SimpleKvStoreKey::try_from(KEY_TX_SIG_LIMIT.as_ref())
                .expect("Unreachable. Const value should be valid"),
            &store,
        );
        let tx_sig_limit = Self::parse_param(raw);

        let raw = Self::get_raw_param(
            SimpleKvStoreKey::try_from(KEY_TX_SIZE_COST_PER_BYTE.as_ref())
                .expect("Unreachable. Const value should be valid"),
            &store,
        );
        let tx_size_cost_per_byte = Self::parse_param(raw);

        let raw = Self::get_raw_param(
            SimpleKvStoreKey::try_from(KEY_SIG_VERIFY_COST_ED25519.as_ref())
                .expect("Unreachable. Const value should be valid"),
            &store,
        );
        let sig_verify_cost_ed25519 = Self::parse_param(raw);

        let raw = Self::get_raw_param(
            SimpleKvStoreKey::try_from(KEY_SIG_VERIFY_COST_SECP256K1.as_ref())
                .expect("Unreachable. Const value should be valid"),
            &store,
        );
        let sig_verify_cost_secp256k1 = Self::parse_param(raw);

        Params {
            max_memo_characters,
            tx_sig_limit,
            tx_size_cost_per_byte,
            sig_verify_cost_ed25519,
            sig_verify_cost_secp256k1,
        }
    }

    pub fn set<DB: Database>(&self, ctx: &mut Context<'_, '_, DB, SK>, params: Params) {
        let mut store = self
            .params_keeper
            .get_mutable_raw_subspace(ctx, &self.params_subspace_key);

        store.set(
            SimpleKvStoreKey::try_from(KEY_MAX_MEMO_CHARACTERS.as_ref())
                .expect("Unreachable. Const value should be valid"),
            format!("\"{}\"", params.max_memo_characters).into(),
        );

        store.set(
            SimpleKvStoreKey::try_from(KEY_TX_SIG_LIMIT.as_ref())
                .expect("Unreachable. Const value should be valid"),
            format!("\"{}\"", params.tx_sig_limit).into(),
        );

        store.set(
            SimpleKvStoreKey::try_from(KEY_TX_SIZE_COST_PER_BYTE.as_ref())
                .expect("Unreachable. Const value should be valid"),
            format!("\"{}\"", params.tx_size_cost_per_byte).into(),
        );

        store.set(
            SimpleKvStoreKey::try_from(KEY_SIG_VERIFY_COST_ED25519.as_ref())
                .expect("Unreachable. Const value should be valid"),
            format!("\"{}\"", params.sig_verify_cost_ed25519).into(),
        );

        store.set(
            SimpleKvStoreKey::try_from(KEY_SIG_VERIFY_COST_SECP256K1.as_ref())
                .expect("Unreachable. Const value should be valid"),
            format!("\"{}\"", params.sig_verify_cost_secp256k1).into(),
        );
    }
}
