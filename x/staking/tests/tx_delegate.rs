use std::str::FromStr;

use gears::{
    core::Protobuf,
    extensions::testing::UnwrapTesting,
    tendermint::types::{
        request::query::RequestQuery,
        response::{ResponseDeliverTx, ResponseQuery},
        time::timestamp::Timestamp,
    },
    types::{address::ValAddress, base::coin::UnsignedCoin, decimal256::Decimal256, uint::Uint256},
    utils::node::{generate_tx, GenesisSource, StepResponse, User},
};
use staking::{
    DelegateMsg, Delegation, DelegationResponse, Message, QueryDelegationRequest,
    QueryDelegationResponse,
};
use utils::{set_node, USER_0, USER_1};

#[path = "./utils.rs"]
mod utils;

const GENESIS_FILE_PATH: &str = "./tests/assets/tx_edit_validator.json";

#[test]
fn create_delegation() {
    let mut node: gears::utils::node::MockNode<
        gears::baseapp::BaseApp<
            gears::store::database::MemDB,
            utils::SubspaceKey,
            utils::MockStakingAbciHandler,
            gears::utils::node::MockApplication,
        >,
        utils::GenesisState,
    > = set_node(GenesisSource::File(GENESIS_FILE_PATH.into()));

    let _ = node.step(vec![], Timestamp::UNIX_EPOCH);

    let user_0 = User::from_bech32(USER_0, 1).unwrap_test();
    let user_1 = User::from_bech32(USER_1, 1).unwrap_test();

    /*
       There is no need to create validator as it already defined in genesis
    */

    // === Delegate

    let msg = Message::Delegate(DelegateMsg {
        validator_address: user_0.address().into(),
        amount: "1000uatom".parse().expect("hardcoded is valid"),
        delegator_address: user_1.address(),
    });

    let txs = generate_tx(vec1::vec1![msg], 0, &user_0, node.chain_id().clone());

    let StepResponse {
        app_hash,
        mut tx_responses,
        height: _,
    } = node.step(vec![txs], Timestamp::UNIX_EPOCH);

    let ResponseDeliverTx { code, log, .. } = tx_responses.pop().unwrap_test();

    assert!(code == 0, "tx log: {log}");

    assert_eq!(
        data_encoding::HEXLOWER.encode(&app_hash),
        "bb5cc2f37bb60c8e29bb338bbf608acd1a8352353132efc5a817c1d03b7d2bae"
    );

    let q = QueryDelegationRequest {
        delegator_addr: user_1.address(),
        validator_addr: user_0.address().into(),
    };

    let ResponseQuery {
        code, value, log, ..
    } = node.query(RequestQuery {
        data: q.encode_vec().into(),
        path: QueryDelegationRequest::QUERY_URL.to_owned(),
        height: node.height() as i64,
        prove: false,
    });

    assert!(code == 0, "{log}");

    let QueryDelegationResponse {
        delegation_response,
    } = QueryDelegationResponse::decode_vec(&value).unwrap_test();

    pretty_assertions::assert_eq!(
        Some(DelegationResponse {
            delegation: Some(Delegation {
                delegator_address: user_1.address(),
                validator_address: ValAddress::from_bech32(
                    "cosmosvaloper1syavy2npfyt9tcncdtsdzf7kny9lh777yfrfs4"
                )
                .unwrap_test(),
                shares: Decimal256::from_atomics(Uint256::from(1000_u16), 0).unwrap_test()
            }),
            balance: Some(UnsignedCoin::from_str("1000uatom").unwrap_test())
        }),
        delegation_response
    );
}

#[test]
fn create_delegation_fails_due_non_existed_validator() {
    let mut node: gears::utils::node::MockNode<
        gears::baseapp::BaseApp<
            gears::store::database::MemDB,
            utils::SubspaceKey,
            utils::MockStakingAbciHandler,
            gears::utils::node::MockApplication,
        >,
        utils::GenesisState,
    > = set_node(GenesisSource::File(GENESIS_FILE_PATH.into()));

    let _ = node.step(vec![], Timestamp::UNIX_EPOCH);

    let user_0 = User::from_bech32(USER_0, 1).unwrap_test();
    let user_1 = User::from_bech32(USER_1, 1).unwrap_test();

    let msg = Message::Delegate(DelegateMsg {
        validator_address: user_1.address().into(),
        amount: "1000uatom".parse().expect("hardcoded is valid"),
        delegator_address: user_0.address(),
    });

    let txs = generate_tx(vec1::vec1![msg], 0, &user_0, node.chain_id().clone());

    let StepResponse {
        app_hash,
        mut tx_responses,
        height: _,
    } = node.step(vec![txs], Timestamp::UNIX_EPOCH);

    let ResponseDeliverTx { code, log, .. } = tx_responses.pop().unwrap_test();

    assert!(code != 0, "tx log: {log}");
    assert_eq!(log, "account not found");

    assert_eq!(
        data_encoding::HEXLOWER.encode(&app_hash),
        "0ecaa53faf2584b6d5af8addc4e8afa8854305cc9cd4276a8106110452cc9828"
    );
}
