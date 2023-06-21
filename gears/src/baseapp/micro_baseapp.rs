use bytes::Bytes;
use core::hash::Hash;
use database::{Database, RocksDB};
use ibc_proto::protobuf::Protobuf;
use proto_messages::cosmos::tx::v1beta1::tx_v2::{self, Message};
use proto_types::AccAddress;
use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};
use store_crate::{MultiStore, StoreKey};
use strum::IntoEnumIterator;
use tendermint_abci::Application;
use tendermint_informal::block::Header;
use tendermint_proto::abci::{RequestDeliverTx, ResponseDeliverTx};
use tracing::info;

use crate::{
    error::AppError,
    types::context_v2::{Context, TxContext},
};

use super::ante_v2::{AnteHandler, AuthKeeper, BankKeeper};

pub trait Handler<M: Message, SK: StoreKey>: Clone + Send + Sync {
    fn handle<DB: Database>(&self, ctx: &mut Context<DB, SK>, msg: &M) -> Result<(), AppError>;
}

#[derive(Debug, Clone)]
pub struct MicroBaseApp<
    S: StoreKey,
    M: Message,
    BK: BankKeeper + Clone + Send + Sync + 'static,
    AK: AuthKeeper + Clone + Send + Sync + 'static,
    H: Handler<M, S>,
> {
    pub multi_store: Arc<RwLock<MultiStore<RocksDB, S>>>,
    base_ante_handler: AnteHandler<BK, AK>,
    handler: H,
    block_header: Arc<RwLock<Option<Header>>>, // passed by Tendermint in call to begin_block
    pub m: PhantomData<M>,
    // pub d: PhantomData<D>,
    // pub r: PhantomData<R>,
}

impl<
        M: Message + 'static,
        // D: Decoder<M> + 'static,
        // R: Router<M> + 'static,
        S: StoreKey + Clone + Send + Sync + 'static,
        BK: BankKeeper + Clone + Send + Sync + 'static,
        AK: AuthKeeper + Clone + Send + Sync + 'static,
        H: Handler<M, S> + 'static,
    > Application for MicroBaseApp<S, M, BK, AK, H>
{
    fn deliver_tx(&self, request: RequestDeliverTx) -> ResponseDeliverTx {
        info!("Got deliver tx request");
        //     match self.run_tx(request.tx) {
        //         Ok(events) => ResponseDeliverTx {
        //             code: 0,
        //             data: Default::default(),
        //             log: "".to_string(),
        //             info: "".to_string(),
        //             gas_wanted: 0,
        //             gas_used: 0,
        //             events: events.into_iter().map(|e| e.into()).collect(),
        //             codespace: "".to_string(),
        //         },
        //         Err(e) => {
        //             info!("Failed to process tx: {}", e);
        //             ResponseDeliverTx {
        //                 code: e.code(),
        //                 data: Bytes::new(),
        //                 log: e.to_string(),
        //                 info: "".to_string(),
        //                 gas_wanted: 0,
        //                 gas_used: 0,
        //                 events: vec![],
        //                 codespace: "".to_string(),
        //             }
        //         }
        //     }
        // let raw = vec![];
        // let msg = D::decode(raw);

        let mut multi_store = self
            .multi_store
            .write()
            .expect("RwLock will not be poisoned");
        let mut ctx = TxContext::new(
            &mut multi_store,
            self.get_block_height(),
            self.get_block_header()
                .expect("block header is set in begin block"),
            request.tx.clone().into(),
        );

        let tx: tx_v2::Tx<M> = tx_v2::Tx::decode(request.tx).unwrap();

        let msgs = tx.get_msgs();

        for msg in msgs {
            self.handler.handle(&mut ctx.as_any(), msg);
        }

        // match Self::run_msgs(&mut ctx.as_any(), tx.get_msgs()) {
        //     Ok(_) => {
        //         let events = ctx.events;
        //         multi_store.write_then_clear_tx_caches();
        //         Ok(events)
        //     }
        //     Err(e) => {
        //         multi_store.clear_tx_caches();
        //         Err(e)
        //     }
        // }

        //self.base_ante_handler.run(ctx, tx);

        // let bank_key = S::get_bank_key();

        // let multi_store = self
        //     .multi_store
        //     .read()
        //     .expect("RwLock will not be poisoned");

        // let bank_store = multi_store.get_kv_store(&bank_key);

        // let signers = msg.get_signers();

        // R::route_msg(msg);
        ResponseDeliverTx::default()
    }
}

impl<
        M: Message,
        // D: Decoder<M>,
        // R: Router<M>,
        S: StoreKey,
        BK: BankKeeper + Clone + Send + Sync + 'static,
        AK: AuthKeeper + Clone + Send + Sync + 'static,
        H: Handler<M, S>,
    > MicroBaseApp<S, M, BK, AK, H>
{
    pub fn new(
        db: RocksDB,
        app_name: &'static str,
        bank_keeper: BK,
        auth_keeper: AK,
        handler: H,
    ) -> Self {
        let multi_store = MultiStore::new(db);
        //let height = multi_store.get_head_version().into();
        Self {
            multi_store: Arc::new(RwLock::new(multi_store)),
            base_ante_handler: AnteHandler::new(bank_keeper, auth_keeper),
            handler,
            block_header: Arc::new(RwLock::new(None)),
            //height: Arc::new(RwLock::new(height)),
            //block_header: Arc::new(RwLock::new(None)),
            //app_name,
            m: PhantomData,
            // d: PhantomData,
            // r: PhantomData,
        }
    }

    fn get_block_height(&self) -> u64 {
        return 42;
    }

    fn get_block_header(&self) -> Option<Header> {
        self.block_header
            .read()
            .expect("RwLock will not be poisoned")
            .clone()
    }

    fn run_msgs<T: Database>(ctx: &mut Context<T, S>, msgs: &Vec<M>) -> Result<(), AppError> {
        // for msg in msgs {
        //     match msg {
        //         Msg::Send(send_msg) => {
        //             Bank::send_coins_from_account_to_account(ctx, send_msg.clone())?
        //         }
        //     };
        // }

        return Ok(());
    }

    //fn run_tx(&self, raw: Bytes) -> Result<Vec<tendermint_informal::abci::Event>, AppError> {}
    // TODO:
    // 1. Check from address is signer + verify signature

    //###########################
    // let tx = Tx::decode(raw.clone()).unwrap();

    // let public = tx.auth_info.clone().unwrap().signer_infos[0]
    //     .clone()
    //     .public_key
    //     .unwrap()
    //     .type_url;
    // println!("################# URL:  {}", public);
    // //cosmos.crypto.secp256k1.PubKey
    // // let msgs = tx.get_msgs();
    // // let msg = &msgs[0];

    // // let signers = msg.get_signers();

    // // println!("################### Signers: {}", signers);

    // // Ok(())

    //     //#######################
    //     let tx = DecodedTx::from_bytes(raw.clone())?;

    //     Self::validate_basic_tx_msgs(tx.get_msgs())?;

    //     let mut multi_store = self
    //         .multi_store
    //         .write()
    //         .expect("RwLock will not be poisoned");
    //     let mut ctx = TxContext::new(
    //         &mut multi_store,
    //         self.get_block_height(),
    //         self.get_block_header()
    //             .expect("block header is set in begin block"),
    //         raw.clone().into(),
    //     );

    //     match AnteHandler::run(&mut ctx.as_any(), &tx) {
    //         Ok(_) => multi_store.write_then_clear_tx_caches(),
    //         Err(e) => {
    //             multi_store.clear_tx_caches();
    //             return Err(e);
    //         }
    //     };

    //     let mut ctx = TxContext::new(
    //         &mut multi_store,
    //         self.get_block_height(),
    //         self.get_block_header()
    //             .expect("block header is set in begin block"),
    //         raw.into(),
    //     );

    //     match Self::run_msgs(&mut ctx.as_any(), tx.get_msgs()) {
    //         Ok(_) => {
    //             let events = ctx.events;
    //             multi_store.write_then_clear_tx_caches();
    //             Ok(events)
    //         }
    //         Err(e) => {
    //             multi_store.clear_tx_caches();
    //             Err(e)
    //         }
    //     }
    // }

    // fn run_msgs<T: DB>(ctx: &mut Context<T>, msgs: &Vec<Msg>) -> Result<(), AppError> {
    //     for msg in msgs {
    //         match msg {
    //             Msg::Send(send_msg) => {
    //                 Bank::send_coins_from_account_to_account(ctx, send_msg.clone())?
    //             }
    //         };
    //     }

    //     return Ok(());
    // }

    // fn validate_basic_tx_msgs(msgs: &Vec<Msg>) -> Result<(), AppError> {
    //     if msgs.is_empty() {
    //         return Err(AppError::InvalidRequest(
    //             "must contain at least one message".into(),
    //         ));
    //     }

    //     for msg in msgs {
    //         msg.validate_basic()
    //             .map_err(|e| AppError::TxValidation(e.to_string()))?
    //     }

    //     return Ok(());
    // }
    // pub fn deliver_tx<M: Message, D: Decoder<M>, R: Router<M>>(
    //     &self,
    //     raw: Vec<u8>,
    // ) -> Result<(), String> {
    //     let msg = D::decode(raw);

    //     let bank_key = S::get_bank_key();

    //     let multi_store = self
    //         .multi_store
    //         .read()
    //         .expect("RwLock will not be poisoned");

    //     let bank_store = multi_store.get_kv_store(&bank_key);

    //     let signers = msg.get_signers();

    //     R::route_msg(msg);
    //     Ok(())
    // }
}
