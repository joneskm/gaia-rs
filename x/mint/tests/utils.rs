use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

use gears::{
    application::handlers::node::ModuleInfo,
    baseapp::BaseApp,
    derive::{ParamsKeys, StoreKeys},
    extensions::{lock::AcquireRwLock, testing::UnwrapTesting},
    store::database::MemDB,
    types::{
        base::{coin::UnsignedCoin, coins::UnsignedCoins},
        decimal256::Decimal256,
        denom::Denom,
    },
    utils::node::{init_node, GenesisSource, MockApplication, MockNode, MockOptions},
    x::{
        keepers::{
            bank::{BalancesKeeper, BankKeeper},
            mint::{MintingBankKeeper, MintingStakingKeeper},
        },
        module::Module,
    },
};
use mint::{abci_handler::MintAbciHandler, genesis::MintGenesis, keeper::MintKeeper};

#[derive(strum::EnumIter, Debug, PartialEq, Eq, Hash, Clone, StoreKeys)]
#[skey(params = Params)]
pub enum SpaceKey {
    #[skey(to_string = "acc")]
    Auth,
    #[skey(to_string = "bank")]
    Bank,
    #[skey(to_string = "mint")]
    Mint,
    #[skey(to_string = "params")]
    Params,
}

#[derive(strum::EnumIter, Debug, PartialEq, Eq, Hash, Clone, ParamsKeys)]
pub enum SubspaceKey {
    #[pkey(to_string = "auth/")]
    Auth,
    #[pkey(to_string = "bank/")]
    Bank,
    #[pkey(to_string = "mint/")]
    Mint,
    #[pkey(to_string = "baseapp/")]
    BaseApp,
}

#[derive(Debug, Clone, PartialEq, Eq, strum::EnumIter)]
pub enum Modules {
    FeeCollector,
    Mint,
}

impl Module for Modules {
    fn name(&self) -> String {
        match self {
            Modules::FeeCollector => "fee_collector".into(),
            Modules::Mint => "mint".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MintModuleInfo;

impl ModuleInfo for MintModuleInfo {
    const NAME: &'static str = "mint";
}

pub fn set_node(
    bank_mock: Option<MockBankKeeper>,
    staking_mock: Option<MockStakingKeeper>,
) -> MockNode<
    BaseApp<
        MemDB,
        SubspaceKey,
        MintAbciHandler<
            SpaceKey,
            SubspaceKey,
            MockBankKeeper,
            MockStakingKeeper,
            Modules,
            MintModuleInfo,
        >,
        MockApplication,
    >,
    MintGenesis,
> {
    let handler = MintAbciHandler::new(
        MintKeeper::new(
            SpaceKey::Mint,
            staking_mock.unwrap_or_default(),
            bank_mock.unwrap_or_default(),
            Modules::Mint,
            Modules::FeeCollector,
        ),
        SubspaceKey::Mint,
    );

    let opt = MockOptions::<
        SubspaceKey,
        MintAbciHandler<_, _, _, _, _, MintModuleInfo>,
        MintGenesis,
    >::former()
    .abci_handler(handler)
    .baseapp_sbs_key(SubspaceKey::BaseApp)
    .genesis(GenesisSource::Default);

    init_node(opt)
}

#[derive(Debug, Clone, Default)]
pub struct MockStakingKeeper {
    pub total_bonded_tokens: Arc<RwLock<Decimal256>>,
}

impl MockStakingKeeper {
    pub fn new(total_bonded_tokens: Decimal256) -> Self {
        Self {
            total_bonded_tokens: Arc::new(RwLock::new(total_bonded_tokens)),
        }
    }
}

impl MintingStakingKeeper<SpaceKey, Modules> for MockStakingKeeper {
    fn staking_denom<
        DB: gears::store::database::Database,
        CTX: gears::context::QueryableContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &CTX,
    ) -> Result<gears::types::denom::Denom, gears::gas::store::errors::GasStoreErrors> {
        Ok(Denom::from_str("uatom").unwrap_test())
    }

    fn total_bonded_tokens<
        DB: gears::store::database::Database,
        CTX: gears::context::QueryableContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &CTX,
    ) -> Result<gears::types::decimal256::Decimal256, gears::gas::store::errors::GasStoreErrors>
    {
        Ok(*self.total_bonded_tokens.acquire_read())
    }
}

#[derive(Debug, Clone, Default)]
pub struct MockBankKeeper {
    pub expected_mint_amount: Option<Arc<RwLock<UnsignedCoins>>>,
    pub supply: Arc<RwLock<Option<UnsignedCoin>>>,
}

impl MockBankKeeper {
    pub fn new(supply: UnsignedCoin, expected_mint: Option<UnsignedCoins>) -> Self {
        Self {
            supply: Arc::new(RwLock::new(Some(supply))),
            expected_mint_amount: expected_mint.map(|this| Arc::new(RwLock::new(this))),
        }
    }
}

impl MintingBankKeeper<SpaceKey, Modules> for MockBankKeeper {
    fn mint_coins<
        DB: gears::store::database::Database,
        CTX: gears::context::TransactionalContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &mut CTX,
        _module: &Modules,
        amount: gears::types::base::coins::UnsignedCoins,
    ) -> Result<(), gears::x::errors::BankKeeperError> {
        if let Some(exp_amount) = &self.expected_mint_amount {
            assert_eq!(exp_amount.acquire_read().clone(), amount)
        }

        Ok(())
    }
}

impl BankKeeper<SpaceKey, Modules> for MockBankKeeper {
    fn send_coins_from_account_to_module<
        DB: gears::store::database::Database,
        CTX: gears::context::TransactionalContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &mut CTX,
        _from_address: gears::types::address::AccAddress,
        _to_module: &Modules,
        _amount: gears::types::base::coins::UnsignedCoins,
    ) -> Result<(), gears::x::errors::BankKeeperError> {
        todo!()
    }

    fn send_coins_from_module_to_account<
        DB: gears::store::database::Database,
        CTX: gears::context::TransactionalContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &mut CTX,
        _address: &gears::types::address::AccAddress,
        _module: &Modules,
        _amount: gears::types::base::coins::UnsignedCoins,
    ) -> Result<(), gears::x::errors::BankKeeperError> {
        todo!()
    }

    fn send_coins_from_module_to_module<
        DB: gears::store::database::Database,
        CTX: gears::context::TransactionalContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &mut CTX,
        _sender_pool: &Modules,
        _recepient_pool: &Modules,
        _amount: gears::types::base::coins::UnsignedCoins,
    ) -> Result<(), gears::x::errors::BankKeeperError> {
        Ok(())
    }

    fn denom_metadata<
        DB: gears::store::database::Database,
        CTX: gears::context::QueryableContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &CTX,
        _base: &gears::types::denom::Denom,
    ) -> Result<
        Option<gears::types::tx::metadata::Metadata>,
        gears::gas::store::errors::GasStoreErrors,
    > {
        todo!()
    }

    fn coins_burn<
        DB: gears::store::database::Database,
        CTX: gears::context::TransactionalContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &mut CTX,
        _module: &Modules,
        _deposit: &gears::types::base::coins::UnsignedCoins,
    ) -> Result<(), gears::x::errors::BankKeeperError> {
        todo!()
    }

    fn send_coins_from_account_to_account<
        DB: gears::store::database::Database,
        CTX: gears::context::TransactionalContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &mut CTX,
        _msg: &gears::types::msg::send::MsgSend,
    ) -> Result<(), gears::x::errors::BankKeeperError> {
        todo!()
    }
}

impl BalancesKeeper<SpaceKey, Modules> for MockBankKeeper {
    fn balance_all<
        DB: gears::store::database::Database,
        CTX: gears::context::QueryableContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &CTX,
        _address: gears::types::address::AccAddress,
        _pagination: Option<gears::extensions::pagination::Pagination>,
    ) -> Result<
        (
            Option<gears::extensions::pagination::PaginationResult>,
            Vec<gears::types::base::coin::UnsignedCoin>,
        ),
        gears::gas::store::errors::GasStoreErrors,
    > {
        todo!()
    }

    fn supply<
        DB: gears::store::database::Database,
        CTX: gears::context::QueryableContext<DB, SpaceKey>,
    >(
        &self,
        _ctx: &CTX,
        _denom: &gears::types::denom::Denom,
    ) -> Result<
        Option<gears::types::base::coin::UnsignedCoin>,
        gears::gas::store::errors::GasStoreErrors,
    > {
        Ok(self.supply.acquire_read().clone())
    }
}
