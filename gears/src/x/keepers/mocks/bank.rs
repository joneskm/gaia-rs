use std::collections::HashMap;

use address::AccAddress;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    context::TransactionalContext,
    types::{
        base::{coin::UnsignedCoin, coins::UnsignedCoins},
        denom::Denom,
        tx::metadata::Metadata,
    },
    x::{
        errors::BankKeeperError,
        keepers::bank::{BalancesKeeper, BankKeeper},
        module::Module,
    },
};

#[derive(former::Former, Clone, Debug)]
pub struct MockBankKeeper {
    pub get_denom_metadata: Option<Metadata>,
    pub balance_all: Vec<UnsignedCoin>,
    pub balance: HashMap<AccAddress, UnsignedCoin>,
    pub supply: HashMap<Denom, UnsignedCoin>,
}

impl<SK: StoreKey, M: Module> BalancesKeeper<SK, M> for MockBankKeeper {
    fn balance_all<DB: database::Database, CTX: crate::context::QueryableContext<DB, SK>>(
        &self,
        _ctx: &CTX,
        _address: address::AccAddress,
        _pagination: Option<extensions::pagination::Pagination>,
    ) -> Result<
        (
            Option<extensions::pagination::PaginationResult>,
            Vec<UnsignedCoin>,
        ),
        GasStoreErrors,
    > {
        Ok((None, self.balance_all.clone()))
    }

    fn supply<DB: database::Database, CTX: crate::context::QueryableContext<DB, SK>>(
        &self,
        _ctx: &CTX,
        denom: &crate::types::denom::Denom,
    ) -> Result<Option<UnsignedCoin>, GasStoreErrors> {
        Ok(self.supply.get(denom).cloned())
    }
}

impl<SK: StoreKey, M: Module> BankKeeper<SK, M> for MockBankKeeper {
    fn send_coins_from_account_to_module<
        DB: database::Database,
        CTX: crate::context::TransactionalContext<DB, SK>,
    >(
        &self,
        _: &mut CTX,
        _: address::AccAddress,
        _: &M,
        _: crate::types::base::coins::UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        Ok(())
    }

    fn send_coins_from_module_to_account<
        DB: database::Database,
        CTX: crate::context::TransactionalContext<DB, SK>,
    >(
        &self,
        _: &mut CTX,
        _: &address::AccAddress,
        _: &M,
        _: crate::types::base::coins::UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        Ok(())
    }

    fn send_coins_from_module_to_module<
        DB: database::Database,
        CTX: TransactionalContext<DB, SK>,
    >(
        &self,
        _: &mut CTX,
        _: &M,
        _: &M,
        _: UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        Ok(())
    }

    fn denom_metadata<DB: database::Database, CTX: crate::context::QueryableContext<DB, SK>>(
        &self,
        _: &CTX,
        _: &crate::types::denom::Denom,
    ) -> Result<Option<Metadata>, GasStoreErrors> {
        Ok(self.get_denom_metadata.clone())
    }

    fn coins_burn<DB: database::Database, CTX: crate::context::TransactionalContext<DB, SK>>(
        &self,
        _: &mut CTX,
        _: &M,
        _: &crate::types::base::coins::UnsignedCoins,
    ) -> Result<(), crate::x::errors::BankKeeperError> {
        Ok(())
    }

    fn send_coins_from_account_to_account<
        DB: database::Database,
        CTX: crate::context::TransactionalContext<DB, SK>,
    >(
        &self,
        _ctx: &mut CTX,
        _msg: &crate::types::msg::send::MsgSend,
    ) -> Result<(), BankKeeperError> {
        Ok(())
    }
}
