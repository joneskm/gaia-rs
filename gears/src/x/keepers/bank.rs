use database::Database;
use kv_store::StoreKey;

use crate::{
    context::{QueryableContext, TransactionalContext},
    types::{
        address::AccAddress,
        base::{coin::UnsignedCoin, coins::UnsignedCoins},
        denom::Denom,
        store::gas::errors::GasStoreErrors,
        tx::metadata::Metadata,
    },
    x::{errors::BankKeeperError, module::Module},
};

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

    fn get_denom_metadata<DB: Database, CTX: QueryableContext<DB, SK>>(
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

// TODO: This trait should be part of staking
/// StakingBankKeeper defines the expected interface needed to retrieve account balances.
pub trait StakingBankKeeper<SK: StoreKey, M: Module>:
    Clone + Send + Sync + 'static + BankKeeper<SK, M>
{
    // GetBalance(ctx sdk.Context, addr sdk.AccAddress, denom string) sdk.Coin
    // LockedCoins(ctx sdk.Context, addr sdk.AccAddress) sdk.Coins
    // SpendableCoins(ctx sdk.Context, addr sdk.AccAddress) sdk.Coins
    //
    // GetSupply(ctx sdk.Context, denom string) sdk.Coin
    //
    // BurnCoins(ctx sdk.Context, name string, amt sdk.Coins) error

    fn get_all_balances<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: AccAddress,
    ) -> Result<Vec<UnsignedCoin>, GasStoreErrors>;

    fn send_coins_from_module_to_module<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        sender_pool: &M,
        recepient_pool: &M,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;

    fn undelegate_coins_from_module_to_account<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        sender_module: &M,
        addr: AccAddress,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;

    fn delegate_coins_from_account_to_module<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        sender_addr: AccAddress,
        recepient_module: &M,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;
}
