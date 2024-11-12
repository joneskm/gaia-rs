use std::{borrow::Cow, num::NonZero};

use crate::{
    baseapp::{errors::QueryError, genesis::Genesis, QueryRequest, QueryResponse},
    context::{block::BlockContext, init::InitContext, query::QueryContext, tx::TxContext},
    types::tx::{raw::TxWithRaw, TxMessage},
};
use database::Database;
use kv_store::StoreKey;
use tendermint::types::{
    proto::validator::ValidatorUpdate,
    request::{begin_block::RequestBeginBlock, end_block::RequestEndBlock, query::RequestQuery},
};
use thiserror::Error;

pub trait ModuleInfo: Clone + Sync + Send + 'static {
    const NAME: &'static str;
}

#[derive(Error, Debug, Clone)]
#[error("{msg}")]
pub struct TxError {
    pub msg: Cow<'static, str>,
    pub code: NonZero<u16>,
    pub codespace: &'static str,
}

impl TxError {
    pub fn new<MI: ModuleInfo>(msg: impl Into<Cow<'static, str>>, code: NonZero<u16>) -> Self {
        Self {
            msg: msg.into(),
            code,
            codespace: MI::NAME,
        }
    }
}

pub trait ABCIHandler: Clone + Send + Sync + 'static {
    type Message: TxMessage;
    type Genesis: Genesis;
    type StoreKey: StoreKey;

    type QReq: QueryRequest;
    type QRes: QueryResponse;

    fn typed_query<DB: Database>(
        &self,
        ctx: &QueryContext<DB, Self::StoreKey>,
        query: Self::QReq,
    ) -> Self::QRes;

    #[allow(unused_variables)]
    fn run_ante_checks<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, Self::StoreKey>,
        tx: &TxWithRaw<Self::Message>,
        is_check: bool,
    ) -> Result<(), TxError> {
        Ok(())
    }

    // TODO: this should return a Result similar to the SDK. See:
    // 1. https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/baseapp/baseapp.go#L717
    // 2. https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/types/abci.pb.go#L323-L333
    // 3. https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/types/result.go#L233-L258
    fn msg<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, Self::StoreKey>,
        msg: &Self::Message,
    ) -> Result<(), TxError>;

    #[allow(unused_variables)]
    fn begin_block<DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, Self::StoreKey>,
        request: RequestBeginBlock,
    ) {
    }

    #[allow(unused_variables)]
    fn end_block<DB: Database>(
        &self,
        ctx: &mut BlockContext<'_, DB, Self::StoreKey>,
        request: RequestEndBlock,
    ) -> Vec<ValidatorUpdate> {
        Vec::new()
    }

    fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, Self::StoreKey>,
        genesis: Self::Genesis,
    ) -> Vec<ValidatorUpdate>;

    fn query<DB: Database + Send + Sync>(
        &self,
        ctx: &QueryContext<DB, Self::StoreKey>,
        query: RequestQuery,
    ) -> Result<Vec<u8>, QueryError>;
}
