use crate::types::{address::AccAddress, base::coins::UnsignedCoins};
use serde::{de::DeserializeOwned, Serialize};

pub use null_genesis::NullGenesis;

#[derive(Debug, Clone, thiserror::Error)]
#[error("cannot add account at existing address {0}")]
pub struct GenesisError(pub AccAddress);

/// Genesis state of application
pub trait Genesis:
    std::fmt::Debug + Default + DeserializeOwned + Serialize + Clone + Send + Sync + 'static
{
    // TODO: REMOVE?
    /// Code to run for each xmod after user added a new genesis account to genesis.
    ///
    /// This method is matter of discussion and alternative should be found. \
    /// See: https://github.com/rumos-io/gears/issues/303
    fn add_genesis_account(
        &mut self,
        _address: AccAddress,
        _coins: UnsignedCoins,
    ) -> Result<(), GenesisError> {
        Ok(())
    }
}

mod null_genesis {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
    pub struct NullGenesis();

    impl Genesis for NullGenesis {
        fn add_genesis_account(
            &mut self,
            _: AccAddress,
            _: UnsignedCoins,
        ) -> Result<(), GenesisError> {
            Ok(())
        }
    }
}
