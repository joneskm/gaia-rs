use cosmwasm_std::Decimal256;
use database::Database;
use gas::store::errors::GasStoreErrors;
use kv_store::StoreKey;
use tendermint::types::proto::validator::VotingPower;

use crate::{
    context::{QueryableContext, TransactionalContext},
    types::{
        address::{AccAddress, ConsAddress, ValAddress},
        base::{coin::UnsignedCoin, coins::UnsignedCoins},
    },
    x::{
        errors::BankKeeperError,
        module::Module,
        types::{delegation::StakingDelegation, validator::StakingValidator},
    },
};

use super::{
    auth::AuthKeeper,
    bank::{BalancesKeeper, BankKeeper},
};

/// Delay, in blocks, between when validator updates are returned to the
/// consensus-engine and when they are applied. For example, if
/// ValidatorUpdateDelay is set to X, and if a validator set update is
/// returned with new validators at the end of block 10, then the new
/// validators are expected to sign blocks beginning at block 11+X.
///
/// This value is constant as this should not change without a hard fork.
/// For Tendermint this should be set to 1 block, for more details see:
/// https://tendermint.com/docs/spec/abci/apps.html#endblock
pub const VALIDATOR_UPDATE_DELAY: u32 = 1;

/// Event Hooks
/// These can be utilized to communicate between a staking keeper and another
/// keeper which must take particular actions when validators/delegators change
/// state. The second keeper must implement this interface, which then the
/// staking keeper can call.
pub trait KeeperHooks<SK: StoreKey, AK: AuthKeeper<SK, M>, M: Module>:
    Clone + Send + Sync + 'static
{
    fn after_validator_created<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        val_addr: ValAddress,
    );

    fn before_validator_modified<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        val_addr: ValAddress,
    );

    fn after_validator_removed<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        cons_addr: ConsAddress,
        val_addr: ValAddress,
    );

    fn after_validator_bonded<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        cons_addr: ConsAddress,
        val_addr: ValAddress,
    );

    fn after_validator_begin_unbonding<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        cons_addr: ConsAddress,
        val_addr: ValAddress,
    );

    fn before_delegation_created<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        del_addr: AccAddress,
        val_addr: ValAddress,
    );

    fn before_delegation_shares_modified<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        del_addr: AccAddress,
        val_addr: ValAddress,
    );

    fn before_delegation_removed<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        del_addr: AccAddress,
        val_addr: ValAddress,
    );

    fn after_delegation_modified<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        del_addr: AccAddress,
        val_addr: ValAddress,
    );

    fn before_validator_slashed<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        val_addr: ValAddress,
        fraction: Decimal256,
    );
}

/// Staking keeper which used in gov xmod
pub trait GovStakingKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    type Validator: StakingValidator;
    type Delegation: StakingDelegation;

    fn bonded_validators_by_power_iter<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<impl Iterator<Item = Result<Self::Validator, GasStoreErrors>>, GasStoreErrors>;

    fn delegations_iter<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        voter: &AccAddress,
    ) -> impl Iterator<Item = Result<Self::Delegation, GasStoreErrors>>;

    fn total_bonded_tokens<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<UnsignedCoin, GasStoreErrors>;
}

/// Staking keeper which used in slashing xmod
pub trait SlashingStakingKeeper<SK: StoreKey, M: Module>: Clone + Send + Sync + 'static {
    type Validator: StakingValidator;
    type Delegation: StakingDelegation;

    /// iterate through validators by operator address, execute func for each validator
    fn validators_iter<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<impl Iterator<Item = Result<Self::Validator, GasStoreErrors>>, GasStoreErrors>;

    /// get a particular validator by operator address
    fn validator<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ValAddress,
    ) -> Result<Option<Self::Validator>, GasStoreErrors>;

    /// get a particular validator by consensus address
    fn validator_by_cons_addr<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &ConsAddress,
    ) -> Result<Option<Self::Validator>, GasStoreErrors>;

    /// slash the validator and delegators of the validator, specifying offence height, offence power, and slash fraction
    fn slash<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &ConsAddress,
        height: u32,
        power: VotingPower,
        slash_fraction_downtime: Decimal256,
    ) -> Result<(), GasStoreErrors>;

    /// jail a validator
    fn jail<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &ConsAddress,
    ) -> Result<(), GasStoreErrors>;

    /// unjail a validator
    fn unjail<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &ConsAddress,
    ) -> Result<(), GasStoreErrors>;

    /// delegation allows for getting a particular delegation for a given validator
    /// and delegator outside the scope of the staking module.
    fn delegation<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        delegator_address: &AccAddress,
        validator_address: &ValAddress,
    ) -> Result<Option<Self::Delegation>, GasStoreErrors>;

    /// max_validators returns the maximum amount of bonded validators
    fn max_validators<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
    ) -> Result<u32, GasStoreErrors>;
    // MaxValidators(sdk.Context) uint32
}

/// Staking keeper which used in distribution xmod
pub trait DistributionStakingKeeper<SK: StoreKey, M: Module>:
    GovStakingKeeper<SK, M> + SlashingStakingKeeper<SK, M>
{
}

/// StakingBankKeeper defines the expected interface needed to retrieve account balances.
pub trait StakingBankKeeper<SK: StoreKey, M: Module>:
    BankKeeper<SK, M> + BalancesKeeper<SK, M> + Clone + Send + Sync + 'static
{
    // GetBalance(ctx sdk.Context, addr sdk.AccAddress, denom string) sdk.Coin
    // LockedCoins(ctx sdk.Context, addr sdk.AccAddress) sdk.Coins
    // SpendableCoins(ctx sdk.Context, addr sdk.AccAddress) sdk.Coins
    //
    // GetSupply(ctx sdk.Context, denom string) sdk.Coin
    //
    // BurnCoins(ctx sdk.Context, name string, amt sdk.Coins) error

    /// Method delegates coins and transfers them from a
    /// delegator account to a module account. It creates the module accounts if it don't exist.
    /// It's safe operation because the modules are app generic parameter
    /// which cannot be added in runtime.

    /// Method undelegates the unbonding coins and transfers
    /// them from a module account to the delegator account.
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
