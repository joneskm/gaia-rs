use database::Database;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    context::{QueryableContext, TransactionalContext},
    types::{account::Account, address::AccAddress},
    x::module::Module,
};

pub trait AuthParams {
    fn max_memo_characters(&self) -> u64;
    fn sig_verify_cost_secp256k1(&self) -> u64;
    fn tx_cost_per_byte(&self) -> u64;
}

pub trait AuthKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    type Params: AuthParams;

    fn get_auth_params<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Self::Params, GasStoreErrors>;

    fn has_account<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &AccAddress,
    ) -> Result<bool, GasStoreErrors>;

    fn get_account<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &AccAddress,
    ) -> Result<Option<Account>, GasStoreErrors>;

    fn set_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        acct: Account,
    ) -> Result<(), GasStoreErrors>;

    /// Overwrites existing account
    fn create_new_base_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &AccAddress,
    ) -> Result<(), GasStoreErrors>;

    /// Creates a new module account if it doesn't already exist
    fn check_create_new_module_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        module: &M,
    ) -> Result<(), GasStoreErrors>;
}
