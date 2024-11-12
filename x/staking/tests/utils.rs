use gears::{
    application::handlers::node::{ABCIHandler, ModuleInfo},
    baseapp::{genesis::GenesisError, BaseApp},
    derive::{ParamsKeys, StoreKeys},
    store::database::MemDB,
    types::{address::AccAddress, base::coins::UnsignedCoins},
    utils::node::{init_node, GenesisSource, MockApplication, MockNode, MockOptionsFormer},
    x::module::Module,
};
use staking::{Keeper, MockHookKeeper, StakingABCIHandler};

/// cosmos1syavy2npfyt9tcncdtsdzf7kny9lh777pahuux
#[allow(dead_code)]
pub const USER_0 : &str = "race draft rival universe maid cheese steel logic crowd fork comic easy truth drift tomorrow eye buddy head time cash swing swift midnight borrow";

/// cosmos15qzm75pjh0jqsv3u40hzp2vzs2hdp47fkz7j5q
#[allow(dead_code)]
pub const USER_1: &str = "unfair live spike near cushion blanket club salad poet cigar venue above north speak harbor salute curve tail appear obvious month end boss priority";

#[allow(dead_code)]
pub const CONSENSUS_KEY: &str = r#"{ "type": "tendermint/PubKeyEd25519", "value": "JVWozgDG2S0TOEE0oFWz/EnSxA0EtYhXQANVIZpePFs="} "#;
#[allow(dead_code)]
pub const CONSENSUS_PUBLIC_KEY : &str = "{\"@type\":\"/cosmos.crypto.ed25519.PubKey\",\"key\":\"JVWozgDG2S0TOEE0oFWz/EnSxA0EtYhXQANVIZpePFs=\"}";

#[derive(Debug, Clone)]
pub struct StakingModuleInfo;

impl ModuleInfo for StakingModuleInfo {
    const NAME: &'static str = "staking";
}

#[derive(Debug, Clone)]
pub struct BankModuleInfo;

impl ModuleInfo for BankModuleInfo {
    const NAME: &'static str = "bank";
}

#[derive(Debug, Clone, PartialEq, Eq, strum::EnumIter)]
pub enum StakingModules {
    FeeCollector,
    BondedPool,
    NotBondedPool,
}

impl Module for StakingModules {
    fn name(&self) -> String {
        match self {
            StakingModules::FeeCollector => "fee_collector".into(),
            StakingModules::BondedPool => staking::BONDED_POOL_NAME.into(),
            StakingModules::NotBondedPool => staking::NOT_BONDED_POOL_NAME.into(),
        }
    }

    fn permissions(&self) -> Vec<String> {
        match self {
            StakingModules::BondedPool => vec!["burner".into(), "staking".into()],
            StakingModules::NotBondedPool => {
                vec!["burner".into(), "staking".into()]
            }
            StakingModules::FeeCollector => Vec::new(),
        }
    }
}

#[derive(strum::EnumIter, Debug, PartialEq, Eq, Hash, Clone, StoreKeys)]
#[skey(params = Params)]
pub enum SpaceKey {
    #[skey(to_string = "acc")]
    Auth,
    #[skey(to_string = "bank")]
    Bank,
    #[skey(to_string = "staking")]
    Staking,
    #[skey(to_string = "params")]
    Params,
}

#[derive(strum::EnumIter, Debug, PartialEq, Eq, Hash, Clone, ParamsKeys)]
pub enum SubspaceKey {
    #[pkey(to_string = "auth/")]
    Auth,
    #[pkey(to_string = "bank/")]
    Bank,
    #[pkey(to_string = "staking/")]
    Staking,
    #[pkey(to_string = "baseapp/")]
    BaseApp,
}

pub fn set_node(
    genesis: GenesisSource<GenesisState>,
) -> MockNode<BaseApp<MemDB, SubspaceKey, MockStakingAbciHandler, MockApplication>, GenesisState> {
    let handler = MockStakingAbciHandler::new();

    let opt: MockOptionsFormer<SubspaceKey, MockStakingAbciHandler, GenesisState> =
        MockOptionsFormer::new()
            .abci_handler(handler)
            .baseapp_sbs_key(SubspaceKey::BaseApp)
            .genesis(genesis);

    init_node(opt)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct GenesisState {
    pub auth: auth::GenesisState,
    pub bank: bank::GenesisState,
    pub staking: staking::GenesisState,
}

impl gears::baseapp::genesis::Genesis for GenesisState {
    fn add_genesis_account(
        &mut self,
        address: AccAddress,
        coins: UnsignedCoins,
    ) -> Result<(), GenesisError> {
        self.bank.add_genesis_account(address.clone(), coins);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MockStakingAbciHandler {
    pub staking: StakingABCIHandler<
        SpaceKey,
        SubspaceKey,
        auth::Keeper<SpaceKey, SubspaceKey, StakingModules>,
        bank::Keeper<
            SpaceKey,
            SubspaceKey,
            auth::Keeper<SpaceKey, SubspaceKey, StakingModules>,
            StakingModules,
        >,
        MockHookKeeper<
            SpaceKey,
            auth::Keeper<SpaceKey, SubspaceKey, StakingModules>,
            StakingModules,
        >,
        StakingModules,
        StakingModuleInfo,
    >,
    pub bank: bank::BankABCIHandler<
        SpaceKey,
        SubspaceKey,
        auth::Keeper<SpaceKey, SubspaceKey, StakingModules>,
        StakingModules,
        BankModuleInfo,
    >,
    pub auth: auth::AuthABCIHandler<SpaceKey, SubspaceKey, StakingModules>,
}

impl Default for MockStakingAbciHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MockStakingAbciHandler {
    pub fn new() -> Self {
        let auth_keeper = auth::Keeper::new(
            SpaceKey::Auth,
            SubspaceKey::Auth,
            StakingModules::FeeCollector,
        );

        let bank_keeper = bank::Keeper::<_, _, _, StakingModules>::new(
            SpaceKey::Bank,
            SubspaceKey::Bank,
            auth_keeper.clone(),
        );

        let auth = auth::AuthABCIHandler::new(auth_keeper.clone());

        let bank = bank::BankABCIHandler::new(bank_keeper.clone());

        let staking = StakingABCIHandler::new(Keeper::new(
            SpaceKey::Staking,
            SubspaceKey::Staking,
            auth_keeper,
            bank_keeper,
            Option::<
                MockHookKeeper<
                    SpaceKey,
                    auth::Keeper<SpaceKey, SubspaceKey, StakingModules>,
                    StakingModules,
                >,
            >::None,
            StakingModules::BondedPool,
            StakingModules::NotBondedPool,
        ));

        Self {
            staking,
            bank,
            auth,
        }
    }
}

impl ABCIHandler for MockStakingAbciHandler {
    type Message = staking::Message;

    type Genesis = GenesisState;

    type StoreKey = SpaceKey;

    type QReq = staking::StakingNodeQueryRequest;

    type QRes = staking::StakingNodeQueryResponse;

    fn typed_query<DB: gears::store::database::Database>(
        &self,
        ctx: &gears::context::query::QueryContext<DB, Self::StoreKey>,
        query: Self::QReq,
    ) -> Self::QRes {
        self.staking.typed_query(ctx, query)
    }

    fn msg<DB: gears::store::database::Database>(
        &self,
        ctx: &mut gears::context::tx::TxContext<'_, DB, Self::StoreKey>,
        msg: &Self::Message,
    ) -> Result<(), gears::application::handlers::node::TxError> {
        self.staking.msg(ctx, msg)
    }

    fn init_genesis<DB: gears::store::database::Database>(
        &self,
        ctx: &mut gears::context::init::InitContext<'_, DB, Self::StoreKey>,
        genesis: Self::Genesis,
    ) -> Vec<gears::tendermint::types::proto::validator::ValidatorUpdate> {
        self.auth.init_genesis(ctx, genesis.auth);
        self.bank.init_genesis(ctx, genesis.bank);
        self.staking.init_genesis(ctx, genesis.staking)
    }

    fn query<DB: gears::store::database::Database + Send + Sync>(
        &self,
        ctx: &gears::context::query::QueryContext<DB, Self::StoreKey>,
        query: gears::tendermint::types::request::query::RequestQuery,
    ) -> Result<Vec<u8>, gears::baseapp::errors::QueryError> {
        self.staking.query(ctx, query)
    }

    fn begin_block<'a, DB: gears::store::database::Database>(
        &self,
        ctx: &mut gears::context::block::BlockContext<'_, DB, Self::StoreKey>,
        request: gears::tendermint::request::RequestBeginBlock,
    ) {
        self.staking.begin_block(ctx, request);
    }

    fn end_block<'a, DB: gears::store::database::Database>(
        &self,
        ctx: &mut gears::context::block::BlockContext<'_, DB, Self::StoreKey>,
        request: gears::tendermint::request::RequestEndBlock,
    ) -> Vec<gears::tendermint::types::proto::validator::ValidatorUpdate> {
        self.staking.end_block(ctx, request)
    }
}
