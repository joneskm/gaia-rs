use std::fmt::Debug;

use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    context::{QueryableContext, TransactionalContext},
    types::account::Account,
    x::{
        keepers::auth::{AuthKeeper, AuthParams},
        module::Module,
    },
};

#[derive(Debug, Clone)]
pub struct MockAuthParams {
    pub max_memo_characters: u64,
    pub sig_verify_cost_secp256k1: u64,
    pub tx_cost_per_byte: u64,
}

impl Default for MockAuthParams {
    fn default() -> Self {
        Self {
            max_memo_characters: 256,
            tx_cost_per_byte: 10,
            sig_verify_cost_secp256k1: 1000,
        }
    }
}

impl AuthParams for MockAuthParams {
    fn max_memo_characters(&self) -> u64 {
        self.max_memo_characters
    }

    fn sig_verify_cost_secp256k1(&self) -> u64 {
        self.sig_verify_cost_secp256k1
    }

    fn tx_cost_per_byte(&self) -> u64 {
        self.tx_cost_per_byte
    }
}

#[derive(former::Former, Clone, Debug)]
pub struct MockAuthKeeper {
    pub get_auth_params: MockAuthParams,
    pub has_account: bool,
    pub get_account: Vec<Account>,
}

impl<SK: StoreKey, M: Module> AuthKeeper<SK, M> for MockAuthKeeper {
    type Params = MockAuthParams;

    fn get_auth_params<DB, CTX: QueryableContext<DB, SK>>(
        &self,
        _: &CTX,
    ) -> Result<Self::Params, GasStoreErrors> {
        Ok(self.get_auth_params.clone())
    }

    fn has_account<DB: database::Database, CTX: QueryableContext<DB, SK>>(
        &self,
        _: &CTX,
        _: &address::AccAddress,
    ) -> Result<bool, GasStoreErrors> {
        Ok(self.has_account)
    }

    fn get_account<DB: database::Database, CTX: QueryableContext<DB, SK>>(
        &self,
        _: &CTX,
        addr: &address::AccAddress,
    ) -> Result<Option<Account>, GasStoreErrors> {
        let account = self
            .get_account
            .iter()
            .find(|this| this.get_address() == addr)
            .cloned();

        Ok(account)
    }

    fn set_account<DB: database::Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        _: &mut CTX,
        _: crate::types::account::Account,
    ) -> Result<(), GasStoreErrors> {
        Ok(())
    }

    fn create_new_base_account<DB: database::Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        _: &mut CTX,
        _: &address::AccAddress,
    ) -> Result<(), GasStoreErrors> {
        Ok(())
    }

    fn check_create_new_module_account<
        DB: database::Database,
        CTX: TransactionalContext<DB, SK>,
    >(
        &self,
        _: &mut CTX,
        _: &M,
    ) -> Result<(), GasStoreErrors> {
        Ok(())
    }
}
