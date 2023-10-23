use bytes::Bytes;
use database::Database;

use gears::{
    baseapp::ante::AuthKeeper,
    error::AppError,
    types::context::{context::Context, init_context::InitContext, query_context::QueryContext},
    x::{auth::Module, params::ParamsSubspaceKey},
};
use ibc_proto::protobuf::Protobuf;
//use params_module::ParamsSubspaceKey;
use prost::Message;
use proto_messages::cosmos::auth::v1beta1::{
    Account, BaseAccount, ModuleAccount, QueryAccountRequest, QueryAccountResponse,
};
use proto_types::AccAddress;
use store::StoreKey;

use crate::{AuthParamsKeeper, GenesisState};

const ACCOUNT_STORE_PREFIX: [u8; 1] = [1];
const GLOBAL_ACCOUNT_NUMBER_KEY: [u8; 19] = [
    103, 108, 111, 098, 097, 108, 065, 099, 099, 111, 117, 110, 116, 078, 117, 109, 098, 101, 114,
]; // "globalAccountNumber"

#[derive(Debug, Clone)]
pub struct Keeper<SK: StoreKey, PSK: ParamsSubspaceKey> {
    store_key: SK,
    auth_params_keeper: AuthParamsKeeper<SK, PSK>,
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey> gears::baseapp::ante::AuthKeeper<SK>
    for Keeper<SK, PSK>
{
    fn get_auth_params<DB: Database>(
        &self,
        ctx: &Context<'_, '_, DB, SK>,
    ) -> gears::x::auth::Params {
        self.auth_params_keeper.get(ctx)
    }

    fn has_account<DB: Database>(&self, ctx: &Context<'_, '_, DB, SK>, addr: &AccAddress) -> bool {
        let auth_store = ctx.multi_store().get_kv_store(&self.store_key);

        let key = create_auth_store_key(addr.to_owned());
        auth_store.get(&key).is_some()
    }

    fn get_account<DB: Database>(
        &self,
        ctx: &Context<'_, '_, DB, SK>,
        addr: &AccAddress,
    ) -> Option<Account> {
        let auth_store = ctx.multi_store().get_kv_store(&self.store_key);

        let key = create_auth_store_key(addr.to_owned());
        let account = auth_store.get(&key);

        if let Some(buf) = account {
            let account = Account::decode::<Bytes>(buf.to_owned().into())
                .expect("invalid data in database - possible database corruption");

            return Some(account);
        }

        return None;
    }

    fn set_account<DB: Database>(&self, ctx: &mut Context<'_, '_, DB, SK>, acct: Account) {
        let auth_store = ctx.multi_store_mut().get_mutable_kv_store(&self.store_key);

        let key = create_auth_store_key(acct.get_address().to_owned());

        auth_store.set(key, acct.encode_vec());
    }
}

impl<SK: StoreKey, PSK: ParamsSubspaceKey> Keeper<SK, PSK> {
    pub fn new(
        store_key: SK,
        params_keeper: gears::x::params::Keeper<SK, PSK>,
        params_subspace_key: PSK,
    ) -> Self {
        let auth_params_keeper = AuthParamsKeeper {
            params_keeper,
            params_subspace_key,
        };
        Keeper {
            store_key,
            auth_params_keeper,
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        genesis: GenesisState,
    ) {
        //TODO: sdk sanitizes accounts
        self.auth_params_keeper.set(&mut ctx.into(), genesis.params);

        for mut acct in genesis.accounts {
            acct.account_number = self.get_next_account_number(&mut ctx.into());
            self.set_account(&mut ctx.into(), Account::Base(acct));
        }

        // Create the fee collector account
        self.check_create_new_module_account(&mut ctx.into(), &Module::FeeCollector);
    }

    pub fn query_account<DB: Database>(
        &self,
        ctx: &QueryContext<'_, DB, SK>,
        req: QueryAccountRequest,
    ) -> Result<QueryAccountResponse, AppError> {
        let auth_store = ctx.get_kv_store(&self.store_key);
        let key = create_auth_store_key(req.address);
        let account = auth_store.get(&key);

        if let Some(buf) = account {
            let account = Account::decode::<Bytes>(buf.to_owned().into())
                .expect("invalid data in database - possible database corruption");

            return Ok(QueryAccountResponse {
                account: account.into(),
            });
        }

        return Err(AppError::AccountNotFound);
    }

    fn get_next_account_number<DB: Database>(&self, ctx: &mut Context<'_, '_, DB, SK>) -> u64 {
        let auth_store = ctx.multi_store_mut().get_mutable_kv_store(&self.store_key);

        // NOTE: The next available account number is what's stored in the KV store
        let acct_num = auth_store.get(&GLOBAL_ACCOUNT_NUMBER_KEY);

        let acct_num: u64 = match acct_num {
            None => 0, //initialize account numbers
            Some(num) => u64::decode::<Bytes>(num.to_owned().into())
                .expect("invalid data in database - possible database corruption"),
        };

        let next_acct_num = acct_num + 1;
        auth_store.set(
            GLOBAL_ACCOUNT_NUMBER_KEY.clone().into(),
            next_acct_num.encode_to_vec(),
        );

        return acct_num;
    }

    pub fn set_account<DB: Database>(&self, ctx: &mut Context<'_, '_, DB, SK>, acct: Account) {
        let auth_store = ctx.multi_store_mut().get_mutable_kv_store(&self.store_key);

        let key = create_auth_store_key(acct.get_address().to_owned());

        auth_store.set(key, acct.encode_vec());
    }

    /// Overwrites existing account
    pub fn create_new_base_account<DB: Database>(
        &self,
        ctx: &mut Context<'_, '_, DB, SK>,
        addr: &AccAddress,
    ) {
        let acct = BaseAccount {
            address: addr.clone(),
            pub_key: None,
            account_number: self.get_next_account_number(ctx),
            sequence: 0,
        };

        self.set_account(ctx, Account::Base(acct))
    }

    /// Creates a new module account if it doesn't already exist
    pub fn check_create_new_module_account<DB: Database>(
        &self,
        ctx: &mut Context<'_, '_, DB, SK>,
        module: &Module,
    ) {
        let addr = module.get_address();

        if self.has_account(ctx, &addr) {
            return;
        } else {
            let account = ModuleAccount {
                base_account: BaseAccount {
                    address: addr.clone(),
                    pub_key: None,
                    account_number: self.get_next_account_number(ctx),
                    sequence: 0,
                },
                name: module.get_name(),
                permissions: module.get_permissions(),
            };

            self.set_account(ctx, Account::Module(account))
        }
    }
}

fn create_auth_store_key(address: AccAddress) -> Vec<u8> {
    let mut auth_store_key: Vec<u8> = address.into();
    let mut prefix = Vec::new();
    prefix.extend(ACCOUNT_STORE_PREFIX);
    prefix.append(&mut auth_store_key);

    return prefix;
}

// TODO: so we really need a Module type?
// pub enum Module {
//     FeeCollector,
// }

// impl Module {
//     pub fn get_address(&self) -> AccAddress {
//         match self {
//             Module::FeeCollector => {
//                 //TODO: construct address from Vec<u8> + make address constant
//                 //TODO: where do these addresses come from?
//                 AccAddress::from_bech32("cosmos17xpfvakm2amg962yls6f84z3kell8c5lserqta")
//                     .expect("hard coded address is valid")
//             }
//         }
//     }

//     pub fn get_name(&self) -> String {
//         match self {
//             Module::FeeCollector => "fee_collector".into(),
//         }
//     }

//     pub fn get_permissions(&self) -> Vec<String> {
//         match self {
//             Module::FeeCollector => vec![],
//         }
//     }
// }

//TODO: copy tests across
