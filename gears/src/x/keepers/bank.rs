use database::Database;
use extensions::pagination::{Pagination, PaginationResult};
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    context::{QueryableContext, TransactionalContext},
    types::{
        address::AccAddress,
        base::{coin::UnsignedCoin, coins::UnsignedCoins},
        denom::Denom,
        msg::send::MsgSend,
        tx::metadata::Metadata,
    },
    x::{errors::BankKeeperError, module::Module},
};

pub trait BalancesKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    fn balance_all<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        address: AccAddress,
        pagination: Option<Pagination>,
    ) -> Result<(Option<PaginationResult>, Vec<UnsignedCoin>), GasStoreErrors>;

    fn supply<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        denom: &Denom,
    ) -> Result<Option<UnsignedCoin>, GasStoreErrors>;
}

pub trait BankKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    fn send_coins_from_account_to_module<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        from_address: AccAddress,
        to_module: &M,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;

    fn send_coins_from_module_to_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &AccAddress,
        module: &M,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;

    fn send_coins_from_account_to_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        msg: &MsgSend,
    ) -> Result<(), BankKeeperError>;

    fn send_coins_from_module_to_module<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        sender_pool: &M,
        recepient_pool: &M,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;

    fn denom_metadata<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        base: &Denom,
    ) -> Result<Option<Metadata>, GasStoreErrors>;

    fn coins_burn<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        module: &M,
        deposit: &UnsignedCoins,
    ) -> Result<(), BankKeeperError>;
}
