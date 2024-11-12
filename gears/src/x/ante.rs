use crate::application::handlers::node::TxError;
use crate::baseapp::options::NodeOptions;
use crate::context::TransactionalContext;
use crate::crypto::public::PublicKey;
use crate::signing::handler::MetadataGetter;
use crate::signing::renderer::amino_renderer::{AminoRenderer, RenderError as AminoRendererError};
use crate::signing::std_sign_doc;
use crate::signing::{handler::SignModeHandler, renderer::value_renderer::ValueRenderer};
use crate::types::base::coin::UnsignedCoin;
use crate::types::base::coins::UnsignedCoins;
use crate::types::denom::Denom;
use crate::x::errors::{AnteError, AnteGasError};
use crate::x::keepers::auth::AuthKeeper;
use crate::x::keepers::auth::AuthParams;
use crate::x::keepers::bank::BankKeeper;
use crate::{
    context::QueryableContext,
    types::tx::{raw::TxWithRaw, signer::SignerData, Tx, TxMessage},
};
use core_types::tx::signature::SignatureData;
use core_types::{
    signing::SignDoc,
    tx::mode_info::{ModeInfo, SignMode},
};
use cosmwasm_std::Decimal256;
use database::Database;
use gas::metering::descriptor::{ANTE_SECKP251K1_DESCRIPTOR, TX_SIZE_DESCRIPTOR};
use gas::metering::kind::TxKind;
use gas::metering::GasMeter;
use gas::store::errors::GasStoreErrors;
use gas::Gas;
use kv_store::StoreKey;
use prost::Message as ProstMessage;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::sync::Arc;

use super::errors::AccountNotFound;
use super::module::Module;

pub trait SignGasConsumer: Clone + Sync + Send + 'static {
    fn consume<AP: AuthParams>(
        &self,
        gas_meter: &mut GasMeter<TxKind>,
        pub_key: PublicKey,
        data: &SignatureData,
        params: &AP,
    ) -> Result<(), GasStoreErrors>;
}

#[derive(Debug, Clone)]
pub struct DefaultSignGasConsumer;

impl SignGasConsumer for DefaultSignGasConsumer {
    fn consume<AP: AuthParams>(
        &self,
        gas_meter: &mut GasMeter<TxKind>,
        pub_key: PublicKey,
        _data: &SignatureData,
        params: &AP,
    ) -> Result<(), GasStoreErrors> {
        // TODO I'm unsure that this is 100% correct due multisig mode see: https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/auth/ante/sigverify.go#L401
        match pub_key {
            PublicKey::Secp256k1(_key) => {
                let amount = params
                    .sig_verify_cost_secp256k1()
                    .try_into()
                    .map_err(|e| GasStoreErrors::new(&[], e))?; // TODO: Should be okay for now, but needs to be changed
                gas_meter
                    .consume_gas(amount, ANTE_SECKP251K1_DESCRIPTOR)
                    .map_err(|e| GasStoreErrors::new(&[], e))?; // TODO: Should be okay for now, but needs to be changed
            }
            PublicKey::Ed25519(_) => todo!(), //TODO: implement
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct BaseAnteHandler<
    BK: BankKeeper<SK, M>,
    AK: AuthKeeper<SK, M>,
    SK: StoreKey,
    GC,
    M: Module,
> {
    bank_keeper: BK,
    auth_keeper: AK,
    sign_gas_consumer: GC,
    fee_collector_module: M,
    sk: PhantomData<SK>,
}

impl<
        AK: AuthKeeper<SK, MOD>,
        BK: BankKeeper<SK, MOD>,
        SK: StoreKey,
        GC: SignGasConsumer,
        MOD: Module,
    > BaseAnteHandler<BK, AK, SK, GC, MOD>
{
    pub fn new(
        auth_keeper: AK,
        bank_keeper: BK,
        sign_gas_consumer: GC,
        fee_collector_module: MOD,
    ) -> BaseAnteHandler<BK, AK, SK, GC, MOD> {
        BaseAnteHandler {
            bank_keeper,
            auth_keeper,
            sign_gas_consumer,
            fee_collector_module,
            sk: PhantomData,
        }
    }
    pub fn run<
        DB: Database,
        M: TxMessage + ValueRenderer + AminoRenderer,
        CTX: TransactionalContext<DB, SK>,
    >(
        &self,
        ctx: &mut CTX,
        tx: &TxWithRaw<M>,
        is_check: bool,
        node_opt: NodeOptions,
        gas_meter: Arc<RefCell<GasMeter<TxKind>>>,
    ) -> Result<(), TxError> {
        // Note: we currently don't have simulate mode at all, so some methods receive hardcoded values for this mode
        // ante.NewSetUpContextDecorator(), // WE not going to implement this in ante. Some logic should be in application
        self.mempool_fee(tx, is_check, node_opt)?;
        self.validate_basic_ante_handler(&tx.tx)?;
        self.tx_timeout_height_ante_handler(ctx, &tx.tx)?;
        self.validate_memo_ante_handler(ctx, &tx.tx)?;
        self.consume_gas_for_tx_size(ctx, tx, gas_meter.clone())?;
        self.deduct_fee_ante_handler(ctx, &tx.tx)?;
        self.set_pub_key_ante_handler(ctx, &tx.tx)?;
        //  ** ante.NewValidateSigCountDecorator(opts.AccountKeeper),
        self.sign_gas_consume(ctx, &tx.tx, gas_meter.clone())?;
        self.sig_verification_handler(ctx, tx)?;
        self.increment_sequence_ante_handler(ctx, &tx.tx)?;
        //  ** ibcante.NewAnteDecorator(opts.IBCkeeper),

        //  - ante.NewRejectExtensionOptionsDecorator(), // Covered in tx parsing code
        //  - NewMempoolFeeDecorator(opts.BypassMinFeeMsgTypes), // NOT USED FOR DELIVER_TX
        //  - ante.NewValidateBasicDecorator(),
        //  - ante.NewTxTimeoutHeightDecorator(),
        //  - ante.NewValidateMemoDecorator(opts.AccountKeeper),
        //  - ante.NewConsumeGasForTxSizeDecorator(opts.AccountKeeper),
        //  - ante.NewDeductFeeDecorator(opts.AccountKeeper, opts.BankKeeper, opts.FeegrantKeeper),
        // // SetPubKeyDecorator must be called before all signature verification decorators
        //  - ante.NewSetPubKeyDecorator(opts.AccountKeeper),
        //  ** ante.NewValidateSigCountDecorator(opts.AccountKeeper),
        //  ante.NewSigGasConsumeDecorator(opts.AccountKeeper, sigGasConsumer),
        //  - ante.NewSigVerificationDecorator(opts.AccountKeeper, opts.SignModeHandler),
        //  - ante.NewIncrementSequenceDecorator(opts.AccountKeeper),
        //  ** ibcante.NewAnteDecorator(opts.IBCkeeper),

        Ok(())
    }

    fn mempool_fee<M: TxMessage>(
        &self,
        TxWithRaw {
            tx,
            raw: _,
            tx_len: _,
        }: &TxWithRaw<M>,
        is_check: bool,
        node_opt: NodeOptions,
    ) -> Result<(), AnteError> {
        if !is_check {
            return Ok(());
        }

        let fee = tx.auth_info.fee.amount.as_ref();
        let gas = tx.auth_info.fee.gas_limit;

        let min_gas_prices = node_opt.min_gas_prices();

        if min_gas_prices.is_empty() || min_gas_prices.is_zero() {
            return Ok(());
        }

        if let Some(fee_coins) = fee {
            let mut required_fees = Vec::with_capacity(min_gas_prices.len());

            for gp in min_gas_prices {
                required_fees.push(UnsignedCoin {
                    denom: gp.denom,
                    amount: gp
                        .amount
                        .checked_mul(Into::<Decimal256>::into(gas))
                        .map_err(|_| {
                            AnteGasError::Overflow("overflow calculating required fees".into())
                        })?
                        .to_uint_ceil(),
                });
            }

            let required_fees = UnsignedCoins::new(required_fees)
                .expect("we know by now that required_fees: contains at least one coin, all amounts are positive, no duplicate denominations and sorted lexicographically");

            if !is_any_gte(fee_coins.inner(), &required_fees) {
                Err(AnteError::InsufficientFees {
                    got: format!("{fee_coins:?}"),
                    required: format!("{required_fees:?}"),
                })?
            }
        } else {
            Err(AnteError::MissingFee)?
        }

        fn is_any_gte(coins_a: &Vec<UnsignedCoin>, coins_b: &UnsignedCoins) -> bool {
            if coins_b.is_empty() {
                return false;
            }

            for coin in coins_a {
                let amount = coins_b.amount_of(&coin.denom);
                if coin.amount >= amount && !amount.is_zero() {
                    return true;
                }
            }

            false
        }

        Ok(())
    }

    fn consume_gas_for_tx_size<M: TxMessage, DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        TxWithRaw {
            tx: _,
            raw: _,
            tx_len,
        }: &TxWithRaw<M>,
        gas_meter: Arc<RefCell<GasMeter<TxKind>>>,
    ) -> Result<(), AnteError> {
        let params = self.auth_keeper.get_auth_params(ctx)?;
        let tx_len: Gas = (*tx_len as u64).try_into().map_err(|_| AnteError::TxLen)?;
        let cost_per_byte: Gas = params.tx_cost_per_byte().try_into().map_err(|_| {
            AnteGasError::Overflow("overflow converting tx cost per byte to gas".to_string())
        })?;
        let gas_required = tx_len
            .checked_mul(cost_per_byte)
            .ok_or(AnteGasError::Overflow(
                "overflow calculating gas required for tx size".to_string(),
            ))?;

        gas_meter
            .borrow_mut()
            .consume_gas(gas_required, TX_SIZE_DESCRIPTOR)
            .map_err(Into::<AnteGasError>::into)?;

        Ok(())
    }

    fn sign_gas_consume<M: TxMessage, DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        tx: &Tx<M>,
        gas_meter: Arc<RefCell<GasMeter<TxKind>>>,
    ) -> Result<(), AnteError> {
        let auth_params = self.auth_keeper.get_auth_params(ctx)?;

        let signatures = tx.get_signatures_data();
        let signers_addr = tx.get_signers();

        for (i, signer_addr) in signers_addr.into_iter().enumerate() {
            let acct = self
                .auth_keeper
                .get_account(ctx, signer_addr)?
                .ok_or(AccountNotFound::from(signer_addr.to_owned()))?;

            let pub_key = acct
                .get_public_key()
                .expect("account pub keys are set in set_pub_key_ante_handler")
                .to_owned();

            let sig = signatures.get(i).expect("TODO"); //TODO: expect message

            self.sign_gas_consumer
                .consume(&mut gas_meter.borrow_mut(), pub_key, sig, &auth_params)
                .map_err(Into::<AnteGasError>::into)?;
        }

        Ok(())
    }

    fn validate_basic_ante_handler<M: TxMessage>(&self, tx: &Tx<M>) -> Result<(), AnteError> {
        // Not sure if we need to explicitly check this given the check which follows.
        // We'll leave it in for now since it's in the SDK.
        let sigs = tx.get_signatures();
        if sigs.is_empty() {
            return Err(AnteError::Validation("signature list is empty".into()));
        }

        if sigs.len() != tx.get_signers().len() {
            return Err(AnteError::Validation(format!(
                "wrong number of signatures; expected {}, got {}",
                tx.get_signers().len(),
                sigs.len()
            )));
        }

        Ok(())
    }

    fn tx_timeout_height_ante_handler<DB: Database, M: TxMessage, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        tx: &Tx<M>,
    ) -> Result<(), AnteError> {
        let timeout_height = tx.get_timeout_height();

        // timeout_height of zero means no timeout height
        if timeout_height == 0 {
            return Ok(());
        }

        let block_height = ctx.height();

        if ctx.height() > timeout_height {
            return Err(AnteError::Timeout {
                timeout: timeout_height,
                current: block_height,
            });
        }

        Ok(())
    }

    fn validate_memo_ante_handler<DB: Database, M: TxMessage, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        tx: &Tx<M>,
    ) -> Result<(), AnteError> {
        let max_memo_chars = self.auth_keeper.get_auth_params(ctx)?.max_memo_characters();
        let memo_length: u64 = tx
            .get_memo()
            .len()
            .try_into()
            .map_err(|_| AnteError::Memo(max_memo_chars))?;

        if memo_length > max_memo_chars {
            return Err(AnteError::Memo(max_memo_chars));
        };
        Ok(())
    }

    fn deduct_fee_ante_handler<DB: Database, M: TxMessage, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        tx: &Tx<M>,
    ) -> Result<(), AnteError> {
        let fee = tx.get_fee();
        let fee_payer = tx.get_fee_payer();

        if !self.auth_keeper.has_account(ctx, fee_payer)? {
            Err(AccountNotFound::from(fee_payer.clone()))?
        }

        if let Some(fee) = fee {
            self.bank_keeper.send_coins_from_account_to_module(
                ctx,
                fee_payer.to_owned(),
                &self.fee_collector_module,
                fee.to_owned(),
            )?;
        }

        Ok(())
    }

    fn set_pub_key_ante_handler<DB: Database, M: TxMessage, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        tx: &Tx<M>,
    ) -> Result<(), AnteError> {
        let public_keys = tx.get_public_keys();
        let signers = tx.get_signers();

        // additional check not found in the sdk - this prevents a panic
        if signers.len() != public_keys.len() {
            return Err(AnteError::Validation(format!(
                "wrong number of signer info; expected {}, got {}",
                signers.len(),
                public_keys.len()
            )));
        }

        for (i, key) in public_keys.into_iter().enumerate() {
            if let Some(key) = key {
                let addr = key.get_address();

                if &addr != signers[i] {
                    return Err(AnteError::Validation(format!(
                        "public key address number {i} does not match signer {i}; expected {}, got {addr}",
                        signers[i]
                    )));
                }

                let mut acct = self
                    .auth_keeper
                    .get_account(ctx, &addr)?
                    .ok_or(AccountNotFound::from(addr.to_owned()))?;

                if acct.get_public_key().is_some() {
                    continue;
                }

                acct.set_public_key(key.clone());
                self.auth_keeper.set_account(ctx, acct)?;
            }
        }

        Ok(())
    }

    fn sig_verification_handler<
        DB: Database,
        M: TxMessage + ValueRenderer + AminoRenderer,
        CTX: TransactionalContext<DB, SK>,
    >(
        &self,
        ctx: &mut CTX,
        tx: &TxWithRaw<M>,
    ) -> Result<(), AnteError> {
        let signers = tx.tx.get_signers();
        let signature_data = tx.tx.get_signatures_data();

        // NOTE: this is also checked in validate_basic_ante_handler
        if signature_data.len() != signers.len() {
            return Err(AnteError::Validation(format!(
                "wrong number of signatures; expected {}, got {}",
                signers.len(),
                signature_data.len()
            )));
        }

        for (i, signature_data) in signature_data.iter().enumerate() {
            let signer = signers[i];

            let acct = self
                .auth_keeper
                .get_account(ctx, signer)?
                .ok_or(AccountNotFound::from(signer.to_owned()))?;

            let account_seq = acct.get_sequence();
            if account_seq != signature_data.sequence {
                return Err(AnteError::Validation(format!(
                    "incorrect tx sequence; expected {}, got {}",
                    account_seq, signature_data.sequence
                )));
            }

            let public_key = acct
                .get_public_key()
                .expect("account pub keys are set in set_pub_key_ante_handler"); //TODO: but can't they be set to None?

            let genesis = ctx.height() == 0;
            let account_number = if genesis {
                0
            } else {
                acct.get_account_number()
            };

            let sign_bytes = match &signature_data.mode_info {
                ModeInfo::Single(mode) => match mode {
                    SignMode::Direct => SignDoc {
                        body_bytes: tx.raw.body_bytes.clone(),
                        auth_info_bytes: tx.raw.auth_info_bytes.clone(),
                        chain_id: ctx.chain_id().to_string(),
                        account_number,
                    }
                    .encode_to_vec(),
                    SignMode::LegacyAminoJson => {
                        let mut msgs = vec![];
                        for msg in tx.tx.get_msgs() {
                            msgs.push(std_sign_doc::Msg {
                                kind: msg.amino_url().to_string(),
                                value: msg.render()?,
                            })
                        }
                        let doc = std_sign_doc::StdSignDoc {
                            account_number: account_number.to_string(),
                            chain_id: ctx.chain_id().to_string(),
                            fee: tx.tx.auth_info.fee.clone().into(),
                            memo: tx.tx.get_memo().to_string(),
                            msgs,
                            sequence: account_seq.to_string(),
                            // TODO: check impl
                            // timeout_height: Some(u64::from(tx.tx.get_timeout_height()).to_string()),
                            timeout_height: None,
                        };

                        doc.to_sign_bytes().map_err(|e| {
                            AnteError::LegacyAminoJson(AminoRendererError::Rendering(e.to_string()))
                        })?
                    }
                    SignMode::Textual => {
                        let handler = SignModeHandler;

                        let signer_data = SignerData {
                            address: signer.to_owned(),
                            chain_id: ctx.chain_id().to_owned(),
                            account_number,
                            sequence: account_seq,
                            pub_key: public_key.to_owned(),
                        };

                        let f = MetadataFromState {
                            bank_keeper: &self.bank_keeper,
                            ctx,
                            _phantom: PhantomData,
                        };

                        handler.sign_bytes_get(&f, signer_data, &tx.tx.body, &tx.tx.auth_info)?
                    }
                    mode => {
                        return Err(AnteError::Validation(format!(
                            "sign mode not supported: {:?}",
                            mode
                        )))
                    }
                },
                ModeInfo::Multi(_) => {
                    return Err(AnteError::Validation("multi sig not supported".to_string()));
                }
            };

            public_key
                .verify_signature(&sign_bytes, &signature_data.signature)
                .map_err(|e| AnteError::Validation(format!("invalid signature: {}", e)))?;
        }

        Ok(())
    }

    fn increment_sequence_ante_handler<
        DB: Database,
        M: TxMessage,
        CTX: TransactionalContext<DB, SK>,
    >(
        &self,
        ctx: &mut CTX,
        tx: &Tx<M>,
    ) -> Result<(), AnteError> {
        for signer in tx.get_signers() {
            let mut acct = self
                .auth_keeper
                .get_account(ctx, signer)?
                .ok_or(AccountNotFound::from(signer.to_owned()))?;
            acct.increment_sequence();
            self.auth_keeper.set_account(ctx, acct)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct MetadataFromState<'a, DB, SK, BK, CTX, MK> {
    pub bank_keeper: &'a BK,
    pub ctx: &'a CTX,
    pub _phantom: PhantomData<(DB, SK, MK)>,
}

impl<
        'a,
        DB: Database,
        SK: StoreKey,
        BK: BankKeeper<SK, M>,
        CTX: QueryableContext<DB, SK>,
        M: Module,
    > MetadataGetter for MetadataFromState<'a, DB, SK, BK, CTX, M>
{
    type Error = GasStoreErrors; // this is not used here

    fn metadata(
        &self,
        denom: &Denom,
    ) -> Result<Option<crate::types::tx::metadata::Metadata>, Self::Error> {
        self.bank_keeper.denom_metadata(self.ctx, denom)
    }
}

// TODO: uncomment tests
// #[cfg(test)]
// mod tests {
//     use database::MemDB;
//     use proto_messages::cosmos::auth::v1beta1::{Account, BaseAccount};
//     use proto_types::AccAddress;
//     use extensions::testing::UnwrapCorrupt;

//     use crate::store::MultiStore;
//     use crate::types::tests::get_signed_tx;
//     use crate::types::InitContext;
//     use crate::x::auth::Auth;

//     use super::*;

//     #[test]
//     fn sig_verification_handler_works() {
//         // TODO: add tests for transactions that are expected to fail
//         let tx = get_signed_tx();

//         let db = MemDB::new();
//         let mut store = MultiStore::new(db);
//         let mut ctx = InitContext::new(&mut store, 0, "unit-testing".into());
//         let account = BaseAccount {
//             address: AccAddress::from_bech32("cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux")
//                 .unwrap_test(),
//             pub_key: None,
//             account_number: 1,
//             sequence: 1,
//         };
//         Auth::set_account(&mut ctx.as_any(), Account::Base(account));
//         set_pub_key_ante_handler(&mut ctx.as_any(), &tx).unwrap_test();
//         sig_verification_handler(&mut ctx.as_any(), &tx).unwrap_test();
//     }
// }
