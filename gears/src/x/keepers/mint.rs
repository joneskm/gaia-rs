use cosmwasm_std::Decimal256;
use database::Database;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    context::{QueryableContext, TransactionalContext},
    types::{base::coins::UnsignedCoins, denom::Denom},
    x::{errors::BankKeeperError, module::Module},
};

use super::bank::{BalancesKeeper, BankKeeper};

pub trait MintingBankKeeper<SK: StoreKey, M: Module>:
    BalancesKeeper<SK, M> + BankKeeper<SK, M> + Clone + Send + Sync + 'static
{
    fn mint_coins<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        module: &M,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError>;
}

pub trait MintingStakingKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    fn staking_denom<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Denom, GasStoreErrors>;

    fn total_bonded_tokens<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<Decimal256, GasStoreErrors>;
}
