use cosmwasm_std::Uint256;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;

use crate::{
    types::base::coin::UnsignedCoin,
    x::{keepers::gov::GovernanceBankKeeper, module::Module},
};

use super::bank::MockBankKeeper;

impl<SK: StoreKey, M: Module> GovernanceBankKeeper<SK, M> for MockBankKeeper {
    fn balance<DB: database::Database, CTX: crate::context::QueryableContext<DB, SK>>(
        &self,
        _: &CTX,
        addr: &address::AccAddress,
        denom: &crate::types::denom::Denom,
    ) -> Result<UnsignedCoin, GasStoreErrors> {
        Ok(self
            .balance
            .get(addr)
            .cloned()
            .unwrap_or_else(|| UnsignedCoin {
                denom: denom.clone(),
                amount: Uint256::zero(),
            }))
    }
}
