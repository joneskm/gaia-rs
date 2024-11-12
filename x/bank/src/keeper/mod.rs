mod mint;
use crate::{Balance, BankParams, BankParamsKeeper};
use bytes::Bytes;
use gears::application::keepers::params::ParamsKeeper;
use gears::context::{init::InitContext, query::QueryContext};
use gears::context::{QueryableContext, TransactionalContext};
use gears::core::Protobuf;
use gears::extensions::corruption::UnwrapCorrupt;
use gears::extensions::gas::GasResultExt;
use gears::extensions::pagination::{IteratorPaginate, Pagination, PaginationResult};
use gears::gas::store::errors::GasStoreErrors;
use gears::params::ParamsSubspaceKey;
use gears::store::database::prefix::PrefixDB;
use gears::store::database::Database;
use gears::store::StoreKey;
use gears::tendermint::types::proto::event::{Event, EventAttribute};
use gears::types::address::AccAddress;
use gears::types::base::coin::UnsignedCoin;
use gears::types::base::coins::{SimpleCoins, UnsignedCoins};
use gears::types::denom::Denom;
use gears::types::msg::send::MsgSend;
use gears::types::store::prefix::mutable::PrefixStoreMut;
use gears::types::tx::metadata::Metadata;
use gears::types::uint::Uint256;
use gears::x::errors::{AccountNotFound, BankCoinsError, BankKeeperError, InsufficientFundsError};
use gears::x::keepers::auth::AuthKeeper;
use gears::x::keepers::bank::{BalancesKeeper, BankKeeper};
use gears::x::keepers::gov::GovernanceBankKeeper;
use gears::x::keepers::staking::StakingBankKeeper;
use gears::x::module::Module;
use std::marker::PhantomData;
use std::ops::SubAssign;
use std::{collections::HashMap, str::FromStr};

pub mod balances;
pub mod bank;
pub mod gov;
pub mod staking;

const SUPPLY_KEY: [u8; 1] = [0];
const ADDRESS_BALANCES_STORE_PREFIX: [u8; 1] = [2];
const DENOM_METADATA_PREFIX: [u8; 1] = [1];

pub(crate) fn account_key(addr: &AccAddress) -> Vec<u8> {
    [
        ADDRESS_BALANCES_STORE_PREFIX.as_slice(),
        &[addr.len()],
        addr.as_ref(),
    ]
    .concat()
}

fn denom_metadata_key(denom: String) -> Vec<u8> {
    [DENOM_METADATA_PREFIX.to_vec(), denom.into_bytes()].concat()
}

fn create_denom_balance_prefix(addr: AccAddress) -> Vec<u8> {
    [
        ADDRESS_BALANCES_STORE_PREFIX.to_vec(),
        [addr.len()].to_vec(),
        addr.into(),
    ]
    .concat()
}

#[derive(Debug, Clone)]
pub struct Keeper<SK: StoreKey, PSK: ParamsSubspaceKey, AK: AuthKeeper<SK, M>, M: Module> {
    store_key: SK,
    bank_params_keeper: BankParamsKeeper<PSK>,
    auth_keeper: AK,
    module_key: PhantomData<M>,
}

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M> + Send + Sync + 'static,
        M: Module,
    > Keeper<SK, PSK, AK, M>
{
    pub fn new(store_key: SK, params_subspace_key: PSK, auth_keeper: AK) -> Self {
        let bank_params_keeper = BankParamsKeeper {
            params_subspace_key,
        };
        Keeper {
            store_key,
            bank_params_keeper,
            auth_keeper,
            module_key: PhantomData,
        }
    }

    pub fn init_genesis<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        mut balances: Vec<Balance>,
        params: BankParams,
        denom_metadata: Vec<Metadata>,
    ) {
        // 1. cosmos SDK sorts the balances first - Make sure that rust ordering gives same result
        // 2. Need to confirm that the SDK does not validate list of coins in each balance (validates order, denom etc.) - Yes it does and our Coins type did it
        // 3. Need to set denom metadata - dedicated cmd for it
        self.bank_params_keeper.set(ctx, params);

        // TODO: This ordering is same as cosmos, but needs to add other constrains like `Coins` type, but with possible empty array
        // TODO: check how it orders if balances are same
        balances.sort_by_key(|this| this.address.clone());

        let mut total_supply: HashMap<Denom, Uint256> = HashMap::new();
        for balance in balances {
            let prefix = create_denom_balance_prefix(balance.address);
            let mut denom_balance_store =
                ctx.kv_store_mut(&self.store_key).prefix_store_mut(prefix);

            for coin in balance.coins {
                denom_balance_store.set(coin.denom.to_string().into_bytes(), coin.encode_vec());
                let zero = Uint256::zero();
                let current_balance = total_supply.get(&coin.denom).unwrap_or(&zero);
                total_supply.insert(coin.denom, coin.amount + current_balance);
            }
        }

        // does the SDK sort these?
        // No. It uses ordering from balances https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/x/bank/keeper/genesis.go#L32-L34
        for (denom, amount) in total_supply {
            self.set_supply(ctx, UnsignedCoin { denom, amount })
                .unwrap_gas();
        }

        for denom_metadata in denom_metadata {
            self.set_denom_metadata(ctx, denom_metadata);
        }
    }

    pub fn params<DB: Database>(&self, ctx: &QueryContext<DB, SK>) -> BankParams {
        self.bank_params_keeper.get(ctx)
    }

    pub fn balance<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        address: &AccAddress,
        denom: &Denom,
    ) -> Result<Option<UnsignedCoin>, GasStoreErrors> {
        let bank_store = ctx.kv_store(&self.store_key);
        let prefix = create_denom_balance_prefix(address.clone());

        let account_store = bank_store.prefix_store(prefix);
        let bal = account_store.get(denom.to_string().as_bytes())?;
        let res = bal.map(|bytes| {
            UnsignedCoin::decode::<Bytes>(bytes.to_owned().into())
                .ok()
                .unwrap_or_corrupt()
        });
        Ok(res)
    }

    /// set_balance sets the coin balance for an account by address.
    pub fn set_balance<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &AccAddress,
        amount: UnsignedCoin,
    ) -> Result<(), GasStoreErrors> {
        let bank_store = ctx.kv_store_mut(&self.store_key);
        let prefix = create_denom_balance_prefix(address.clone());

        let mut account_store = bank_store.prefix_store_mut(prefix);
        if amount.amount.is_zero() {
            account_store.delete(amount.denom.to_string().as_bytes())?;
            Ok(())
        } else {
            account_store.set(
                amount.denom.to_string().as_bytes().to_vec(),
                amount.encode_vec(),
            )
        }
    }

    /// add_coins increase the addr balance by the given amt. Fails if the provided amt is invalid.
    /// It emits a coin received event.
    pub fn add_coins<'a, DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        address: &AccAddress,
        amount: impl IntoIterator<Item = &'a UnsignedCoin>,
    ) -> Result<(), GasStoreErrors> {
        let amount = amount.into_iter().collect::<Vec<_>>();

        for coin in &amount {
            if let Some(mut balance) = self.balance(ctx, address, &coin.denom)? {
                balance.amount += coin.amount;
                self.set_balance(ctx, address, balance)?;
            } else {
                self.set_balance(ctx, address, (**coin).clone())?;
            }
        }

        // emit coin received event
        ctx.push_event(Event::new(
            "coin_received",
            [
                EventAttribute::new(
                    "receiver".into(),
                    String::from(address.clone()).into(),
                    true,
                ),
                EventAttribute::new(
                    "amount".into(),
                    gears::types::base::coins::format_coins(amount),
                    true,
                ),
            ],
        ));
        Ok(())
    }

    /// Gets the total supply of every denom
    pub fn total_supply<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        pagination: Option<Pagination>,
    ) -> (Option<PaginationResult>, Vec<UnsignedCoin>) {
        let bank_store = ctx.kv_store(&self.store_key);
        let supply_store = bank_store.prefix_store(SUPPLY_KEY);

        let supply_store = supply_store
            .into_range(..)
            .map(|raw_coin| {
                let denom = Denom::from_str(&String::from_utf8_lossy(&raw_coin.0))
                    .ok()
                    .unwrap_or_corrupt();
                let amount = Uint256::from_str(&String::from_utf8_lossy(&raw_coin.1))
                    .ok()
                    .unwrap_or_corrupt();
                UnsignedCoin { denom, amount }
            })
            .filter(|this| !this.amount.is_zero());

        let (p_result, iter) = supply_store.maybe_paginate(pagination);

        let mut store: Vec<_> = iter.collect();

        store.sort_by_key(|this| this.denom.clone());

        (p_result, store)
    }

    fn send_coins<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        MsgSend {
            from_address,
            to_address,
            amount,
        }: MsgSend,
    ) -> Result<(), BankKeeperError> {
        if let Some(denom) = self.find_first_blocked_denom_if_any(ctx, amount.inner())? {
            Err(BankKeeperError::SendDisabled(denom.clone()))?
        }

        let mut events = vec![];

        for send_coin in amount.inner() {
            let mut from_account_store = self.address_balances_store(ctx, &from_address);
            let from_balance = from_account_store
                .get(send_coin.denom.to_string().as_bytes())?
                .ok_or(InsufficientFundsError::RequiredActual {
                    required: send_coin.amount,
                    actual: Uint256::zero(),
                })?;

            let mut from_balance: UnsignedCoin =
                UnsignedCoin::decode::<Bytes>(from_balance.to_owned().into())
                    .ok()
                    .unwrap_or_corrupt();

            if from_balance.amount < send_coin.amount {
                Err(InsufficientFundsError::RequiredActual {
                    required: send_coin.amount,
                    actual: from_balance.amount,
                })?;
            }

            from_balance.amount -= send_coin.amount;

            // if balance == 0 then denom should be removed from store
            if from_balance.amount.is_zero() {
                from_account_store.delete(send_coin.denom.to_string().as_bytes())?;
            } else {
                from_account_store.set(
                    send_coin.denom.clone().to_string().into_bytes(),
                    from_balance.encode_vec(),
                )?;
            }
        }

        for send_coin in amount.inner() {
            let mut to_account_store = self.address_balances_store(ctx, &to_address);
            let to_balance = to_account_store.get(send_coin.denom.to_string().as_bytes())?;

            let mut to_balance: UnsignedCoin = match to_balance {
                Some(to_balance) => UnsignedCoin::decode::<Bytes>(to_balance.to_owned().into())
                    .ok()
                    .unwrap_or_corrupt(),
                None => UnsignedCoin {
                    denom: send_coin.denom.clone(),
                    amount: Uint256::zero(),
                },
            };

            to_balance.amount += send_coin.amount;

            to_account_store.set(
                send_coin.denom.to_string().into_bytes(),
                to_balance.encode_vec(),
            )?;

            events.push(Event::new(
                "transfer",
                [
                    EventAttribute::new(
                        "recipient".into(),
                        String::from(to_address.clone()).into(),
                        true,
                    ),
                    EventAttribute::new(
                        "sender".into(),
                        String::from(from_address.clone()).into(),
                        true,
                    ),
                    EventAttribute::new("amount".into(), send_coin.amount.to_string().into(), true),
                ],
            ));
        }

        ctx.append_events(events);

        Ok(())
    }

    pub fn set_supply<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        coin: UnsignedCoin,
    ) -> Result<(), GasStoreErrors> {
        let bank_store = ctx.kv_store_mut(&self.store_key);
        let mut supply_store = bank_store.prefix_store_mut(SUPPLY_KEY);

        match coin.amount.is_zero() {
            true => supply_store
                .delete(coin.denom.to_string().as_bytes())
                .map(|_| ()),
            false => supply_store.set(
                coin.denom.to_string().into_bytes(),
                coin.amount.to_string().into_bytes(),
            ),
        }
    }

    fn address_balances_store<'a, DB: Database>(
        &'a self,
        ctx: &'a mut impl TransactionalContext<DB, SK>,
        address: &AccAddress,
    ) -> PrefixStoreMut<'a, PrefixDB<DB>> {
        let prefix = create_denom_balance_prefix(address.to_owned());
        let bank_store = ctx.kv_store_mut(&self.store_key);
        bank_store.prefix_store_mut(prefix)
    }

    /// Sets the denominations metadata
    pub fn set_denom_metadata<DB: Database>(
        &self,
        ctx: &mut InitContext<'_, DB, SK>,
        denom_metadata: Metadata,
    ) {
        // NOTE: we use the denom twice, once for the prefix and once for the key.
        // This seems unnecessary, I'm not sure why they do this in the SDK.
        let bank_store = ctx.kv_store_mut(&self.store_key);
        let mut denom_metadata_store =
            bank_store.prefix_store_mut(denom_metadata_key(denom_metadata.base.clone()));

        denom_metadata_store.set(
            denom_metadata.base.clone().into_bytes(),
            denom_metadata.encode_vec(),
        );
    }

    pub fn denoms_metadata<DB: Database>(
        &self,
        ctx: &QueryContext<DB, SK>,
        pagination: Option<Pagination>,
    ) -> (Option<PaginationResult>, Vec<Metadata>) {
        let bank_store = ctx.kv_store(&self.store_key);
        let mut denoms_metadata = vec![];

        let bank_iterator = bank_store
            .clone()
            .prefix_store(DENOM_METADATA_PREFIX)
            .into_range(..);

        let (p_result, iter) = bank_iterator.maybe_paginate(pagination);

        for (_, metadata) in iter {
            let metadata: Metadata = Metadata::decode::<Bytes>(metadata.into_owned().into())
                .ok()
                .unwrap_or_corrupt();
            denoms_metadata.push(metadata);
        }

        (p_result, denoms_metadata)
    }

    /// delegate_coins performs delegation by deducting amt coins from an account with
    /// address addr. For vesting accounts, delegations amounts are tracked for both
    /// vesting and vested coins. The coins are then transferred from the delegator
    /// address to a ModuleAccount address. If any of the delegation amounts are negative,
    /// an error is returned.
    fn delegate_coins<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        delegator_addr: AccAddress,
        module_acc_addr: AccAddress,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        if self
            .auth_keeper
            .get_account(ctx, &module_acc_addr)?
            .is_none()
        {
            Err(AccountNotFound::from(module_acc_addr.to_owned()))?
        };

        let mut balances = vec![];
        for coin in amount.inner() {
            if let Some(mut balance) = self.balance(ctx, &delegator_addr, &coin.denom)? {
                if balance.amount < coin.amount {
                    return Err(BankKeeperError::Delegation {
                        smaller: balance.amount,
                        bigger: coin.amount,
                    });
                }
                balances.push(balance.clone());
                balance.amount -= coin.amount;
                self.set_balance(ctx, &delegator_addr, balance)?;
            } else {
                return Err(BankKeeperError::Delegation {
                    smaller: Uint256::zero(),
                    bigger: coin.amount,
                });
            }
        }

        self.track_delegation(
            ctx,
            &delegator_addr,
            &UnsignedCoins::new(balances.clone())?,
            &amount,
        )?;

        // emit coin spent event
        ctx.push_event(Event::new(
            "coin_spent",
            [
                EventAttribute::new("spender".into(), String::from(delegator_addr).into(), true),
                EventAttribute::new(
                    "amount".into(),
                    gears::types::base::coins::format_coins(amount.inner()),
                    true,
                ),
            ],
        ));

        Ok(self.add_coins(ctx, &module_acc_addr, amount.inner())?)
    }

    /// undelegate_coins performs undelegation by crediting amt coins to an account with
    /// address addr. For vesting accounts, undelegation amounts are tracked for both
    /// vesting and vested coins. The coins are then transferred from a ModuleAccount
    /// address to the delegator address. If any of the undelegation amounts are
    /// negative, an error is returned.
    fn undelegate_coins<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        module_acc_addr: AccAddress,
        delegator_addr: AccAddress,
        amount: UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        if self
            .auth_keeper
            .get_account(ctx, &module_acc_addr)?
            .is_none()
        {
            Err(AccountNotFound::from(module_acc_addr.to_owned()))?
        };

        self.sub_unlocked_coins(ctx, &module_acc_addr, &amount)?;
        self.track_undelegation(ctx, &delegator_addr, &amount)?;
        Ok(self.add_coins(ctx, &delegator_addr, amount.inner())?)
    }

    /// sub_unlocked_coins removes the unlocked amt coins of the given account. An error is
    /// returned if the resulting balance is negative. A coin_spent event is emitted after.
    fn sub_unlocked_coins<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &AccAddress,
        amount: &UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        let locked_coins = self.locked_coins(ctx, addr)?;

        let amount_of = |coins: &Vec<UnsignedCoin>, denom: &Denom| -> Uint256 {
            let coins = coins.iter().find(|c| c.denom == *denom);
            coins.map(|c| c.amount).unwrap_or(Uint256::zero())
        };

        for coin in amount.inner() {
            if let Some(mut balance) = self.balance(ctx, addr, &coin.denom)? {
                let locked_amount = amount_of(&locked_coins, &coin.denom);
                let spendable = balance.amount - locked_amount;

                if spendable.checked_sub(coin.amount).is_err() {
                    Err(BankCoinsError::Amount {
                        smaller: spendable,
                        bigger: coin.amount,
                    })?;
                }

                balance.amount -= coin.amount;
                self.set_balance(ctx, addr, balance)?;
            } else {
                Err(InsufficientFundsError::Account {
                    account: addr.clone(),
                    funds: coin.denom.clone(),
                })?;
            }
        }

        // emit coin spent event
        ctx.push_event(Event::new(
            "coin_spent",
            [
                EventAttribute::new("spender".into(), String::from(addr.clone()).into(), true),
                EventAttribute::new(
                    "amount".into(),
                    SimpleCoins::new(amount.clone()).to_string_bytes(),
                    true,
                ),
            ],
        ));
        Ok(())
    }

    /// locked_coins returns all the coins that are not spendable (i.e. locked) for an
    /// account by address. For standard accounts, the result will always be no coins.
    /// For vesting accounts, locked_coins is delegated to the concrete vesting account
    /// type.
    fn locked_coins<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &AccAddress,
    ) -> Result<Vec<UnsignedCoin>, BankKeeperError> {
        if let Some(_acc) = self.auth_keeper.get_account(ctx, addr)? {
            //     vacc, ok := acc.(vestexported.VestingAccount)
            //     if ok {
            //         return vacc.LockedCoins(ctx.BlockTime())
            //     }
            // TODO: logic with vesting accounts
            Ok(vec![])
        } else {
            Ok(vec![])
        }
    }

    /// track_delegation tracks the delegation of the given account if it is a vesting account
    fn track_delegation<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &AccAddress,
        _balance: &UnsignedCoins,
        _amount: &UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        if let Some(_acc) = self.auth_keeper.get_account(ctx, addr)? {
            // TODO: logic with vesting accounts
            //     vacc, ok := acc.(vestexported.VestingAccount)
            //     if ok {
            //         vacc.TrackDelegation(ctx.BlockHeader().Time, balance, amt)
            //         k.ak.SetAccount(ctx, acc)
            //     }
            Ok(())
        } else {
            Err(AccountNotFound::from(addr.to_owned()))?
        }
    }

    /// track_undelegation trakcs undelegation of the given account if it is a vesting account
    fn track_undelegation<DB: Database, CTX: TransactionalContext<DB, SK>>(
        &self,
        ctx: &mut CTX,
        addr: &AccAddress,
        _amount: &UnsignedCoins,
    ) -> Result<(), BankKeeperError> {
        if let Some(_acc) = self.auth_keeper.get_account(ctx, addr)? {
            // TODO: logic with vesting accounts
            //     vacc, ok := acc.(vestexported.VestingAccount)
            //     if ok {
            //         vacc.TrackUndelegation(amt)
            //         k.ak.SetAccount(ctx, acc)
            //     }
            Ok(())
        } else {
            Err(AccountNotFound::from(addr.to_owned()))?
        }
    }

    /// returns the coins the given address can spend alongside the total amount of coins it holds.
    /// It exists for gas efficiency, in order to avoid to have to get balance multiple times.
    pub fn spendable_coins<DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        addr: &AccAddress,
        pagination: Option<Pagination>,
    ) -> Result<
        (
            Option<UnsignedCoins>,
            UnsignedCoins,
            Option<PaginationResult>,
        ),
        BankKeeperError,
    > {
        let (pagination, total) = self.balance_all(ctx, addr.clone(), pagination)?;
        let locked = self.locked_coins(ctx, addr)?;

        let total = UnsignedCoins::new(total)?;
        let locked = UnsignedCoins::new(locked)?;

        match total.checked_sub(&locked) {
            Ok(spendable) => Ok((Some(spendable), total, pagination)),
            Err(_) => Ok((None, total, pagination)),
        }
    }

    fn find_first_blocked_denom_if_any<'a, DB: Database, CTX: QueryableContext<DB, SK>>(
        &self,
        ctx: &CTX,
        coins: impl IntoIterator<Item = &'a UnsignedCoin>,
    ) -> Result<Option<&'a Denom>, GasStoreErrors> {
        let BankParams {
            send_enabled,
            default_send_enabled,
        } = self.bank_params_keeper.try_get(ctx)?;

        let send_enabled = send_enabled
            .into_iter()
            .map(|this| (this.denom, this.enabled))
            .collect::<HashMap<_, _>>();
        for UnsignedCoin { denom, amount: _ } in coins {
            let enabled = send_enabled
                .get(denom)
                .copied()
                .unwrap_or(default_send_enabled);

            if !enabled {
                return Ok(Some(denom));
            }
        }

        Ok(None)
    }
}

//TODO: copy tests across
