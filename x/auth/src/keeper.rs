use crate::{AuthParamsKeeper, AuthsParams};

use bytes::Bytes;
use gears::context::init::InitContext;
use gears::extensions::gas::GasResultExt;
use gears::gas::store::errors::GasStoreErrors;
use prost::Message;

use gears::application::keepers::params::ParamsKeeper;
use gears::context::query::QueryContext;
use gears::context::{QueryableContext, TransactionalContext};
use gears::core::Protobuf as _;
use gears::extensions::corruption::UnwrapCorrupt;
use gears::extensions::pagination::{IteratorPaginate, Pagination, PaginationResult};
use gears::params::ParamsSubspaceKey;
use gears::store::database::Database;
use gears::store::StoreKey;
use gears::types::account::{Account, BaseAccount, ModuleAccount};
use gears::types::address::AccAddress;
use gears::x::keepers::auth::AuthKeeper;
use gears::x::module::Module;

const ACCOUNT_STORE_PREFIX: [u8; 1] = [1];
const GLOBAL_ACCOUNT_NUMBER_KEY: [u8; 19] = [
    103, 108, 111, 98, 97, 108, 65, 99, 99, 111, 117, 110, 116, 78, 117, 109, 98, 101, 114,
]; // "globalAccountNumber"

#[derive(Debug, Clone)]
pub struct Keeper<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module> {
    store_key: SK,
    auth_params_keeper: AuthParamsKeeper<PSK>,
    /// Fee collector access key.
    fee_collector_module: M,
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module> AuthKeeper<SK, M> for Keeper<SK, PSK, M> {
    type Params = AuthsParams;

    fn get_auth_params<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Self::Params, GasStoreErrors> {
        self.auth_params_keeper.try_get(ctx)
    }

    fn has_account<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &AccAddress,
    ) -> Result<bool, GasStoreErrors> {
        let auth_store = ctx.kv_store(&self.store_key);
        let key = create_auth_store_key(addr.to_owned());
        Ok(auth_store.get(&key)?.is_some())
    }

    fn get_account<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &AccAddress,
    ) -> Result<Option<Account>, GasStoreErrors> {
        let auth_store = ctx.kv_store(&self.store_key);
        let key = create_auth_store_key(addr.to_owned());
        let account = auth_store.get(&key)?;

        if let Some(buf) = account {
            let account = Account::decode::<Bytes>(buf.to_owned().into())
                .ok()
                .unwrap_or_corrupt();

            Ok(Some(account))
        } else {
            Ok(None)
        }
    }

    fn set_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        acct: Account,
    ) -> Result<(), GasStoreErrors> {
        let mut auth_store = ctx.kv_store_mut(&self.store_key);
        let key = create_auth_store_key(acct.get_address().to_owned());

        auth_store.set(key, acct.encode_vec())?;

        Ok(())
    }

    fn create_new_base_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &AccAddress,
    ) -> Result<(), GasStoreErrors> {
        let acct = BaseAccount {
            address: addr.clone(),
            pub_key: None,
            account_number: next_account_number(&self.store_key, ctx)?,
            sequence: 0,
        };

        self.set_account(ctx, Account::Base(acct))?;

        Ok(())
    }

    fn check_create_new_module_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        module: &M,
    ) -> Result<(), GasStoreErrors> {
        let addr = module.address();

        if !self.has_account(ctx, &addr)? {
            let account = ModuleAccount {
                base_account: BaseAccount {
                    address: addr,
                    pub_key: None,
                    account_number: next_account_number(&self.store_key, ctx)?,
                    sequence: 0,
                },
                name: module.name(),
                permissions: module.permissions().into_iter().collect(),
            };

            self.set_account(ctx, Account::Module(account))?
        }
        Ok(())
    }
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey, M: Module> Keeper<SK, PSK, M> {
    pub fn new(store_key: SK, params_subspace_key: PSK, fee_collector_module: M) -> Self {
        let auth_params_keeper = AuthParamsKeeper {
            params_subspace_key,
        };

        Keeper {
            store_key,
            auth_params_keeper,
            fee_collector_module,
        }
    }

    pub fn init<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        mut accounts: Vec<Account>,
        params: AuthsParams,
    ) {
        self.auth_params_keeper.set(ctx, params);

        // sanitizing
        accounts.sort_by_key(|a| a.get_account_number());

        for mut acct in accounts {
            acct.set_account_number(next_account_number(&self.store_key, ctx).unwrap_gas());
            self.set_account(ctx, acct).unwrap_gas();
        }

        // Create the fee collector account
        self.check_create_new_module_account(ctx, &self.fee_collector_module)
            .unwrap_gas();
    }

    pub fn accounts<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        pagination: Option<Pagination>,
    ) -> (Option<PaginationResult>, Vec<Account>) {
        let auth_store = ctx.kv_store(&self.store_key);
        let auth_store = auth_store.prefix_store(ACCOUNT_STORE_PREFIX);
        let (p_res, iter) = auth_store.into_range(..).maybe_paginate(pagination);

        (
            p_res,
            iter.map(|(_k, bytes)| Account::decode_vec(&bytes).unwrap_or_corrupt())
                .collect(),
        )
    }
}

fn create_auth_store_key(address: AccAddress) -> Vec<u8> {
    [ACCOUNT_STORE_PREFIX.to_vec(), Vec::<u8>::from(address)].concat()
}

fn next_account_number<SK: StoreKey, DB: Database, CTX: TransactionalContext<DB, SK>>(
    sk: &SK,
    ctx: &mut CTX,
) -> Result<u64, GasStoreErrors> {
    let mut auth_store = ctx.kv_store_mut(sk);

    // NOTE: The next available account number is what's stored in the KV store
    let acct_num = auth_store.get(&GLOBAL_ACCOUNT_NUMBER_KEY)?;

    let acct_num: u64 = match acct_num {
        None => 0, //initialize account numbers
        Some(num) => u64::decode(Bytes::copy_from_slice(&num))
            .ok()
            .unwrap_or_corrupt(),
    };

    let next_acct_num = acct_num + 1;
    auth_store.set(GLOBAL_ACCOUNT_NUMBER_KEY, next_acct_num.encode_to_vec())?;

    Ok(acct_num)
}
