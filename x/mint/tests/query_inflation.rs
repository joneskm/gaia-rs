use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

use gears::{
    core::Protobuf,
    extensions::{lock::AcquireRwLock, testing::UnwrapTesting},
    tendermint::types::{
        request::query::RequestQuery,
        response::ResponseQuery,
        time::{
            duration::Duration,
            timestamp::{Timestamp, TimestampSeconds},
        },
    },
    types::{base::coin::UnsignedCoin, decimal256::Decimal256, uint::Uint256},
};
use mint::types::query::{request::QueryInflationRequest, response::QueryInflationResponse};
use utils::{set_node, MockBankKeeper, MockStakingKeeper};

#[path = "./utils.rs"]
mod utils;

#[test]
fn query_inflation_after_init_without_staking_supply() {
    let mut node = set_node(None, None);

    let _ = node.step(vec![], Timestamp::UNIX_EPOCH);

    let q = QueryInflationRequest {};
    let ResponseQuery { value, .. } = node.query(RequestQuery {
        data: q.encode_vec().into(),
        path: QueryInflationRequest::QUERY_URL.to_owned(),
        height: 1,
        prove: false,
    });

    let QueryInflationResponse { inflation } =
        QueryInflationResponse::decode_vec(&value).unwrap_test();

    let expected_inflation = Decimal256::from_atomics(2_u8, 1).unwrap_test();

    assert_eq!(expected_inflation, inflation);
}

#[test]
fn query_inflation_after_init() {
    let mut node = set_node(
        Some(MockBankKeeper::new(
            UnsignedCoin::from_str("1000000000000uatom").unwrap_test(),
            None,
        )),
        Some(MockStakingKeeper::new(Decimal256::new(Uint256::from(
            1000000000_u64,
        )))),
    );

    let _ = node.step(vec![], Timestamp::UNIX_EPOCH);

    let q = QueryInflationRequest {};
    let ResponseQuery { value, .. } = node.query(RequestQuery {
        data: q.encode_vec().into(),
        path: QueryInflationRequest::QUERY_URL.to_owned(),
        height: 1,
        prove: false,
    });

    let QueryInflationResponse { inflation } =
        QueryInflationResponse::decode_vec(&value).unwrap_test();

    let expected_inflation = Decimal256::from_atomics(2_u8, 1).unwrap_test();

    assert_eq!(expected_inflation, inflation);
}

#[test]
fn query_inflation_after_month_without_staking_supply() {
    let mut node = set_node(None, None);

    // Well. I simulate chain which runs for month and each block takes 5 seconds
    let mut timestamp = Timestamp::UNIX_EPOCH;
    while timestamp.timestamp_seconds() <= TimestampSeconds::try_from(2_628_000).unwrap_test() {
        let _ = node.step(vec![], timestamp);

        timestamp = timestamp
            .checked_add(Duration::new_from_secs(5))
            .unwrap_test();
    }

    let q = QueryInflationRequest {};
    let ResponseQuery { value, .. } = node.query(RequestQuery {
        data: q.encode_vec().into(),
        path: QueryInflationRequest::QUERY_URL.to_owned(),
        height: node.height() as i64,
        prove: false,
    });

    let QueryInflationResponse { inflation } =
        QueryInflationResponse::decode_vec(&value).unwrap_test();

    let expected_inflation = Decimal256::from_atomics(2_u8, 1).unwrap_test();

    assert_eq!(expected_inflation, inflation);
}

#[test]
fn query_inflation_after_month() {
    let total_supply = Arc::new(RwLock::new(Some(
        UnsignedCoin::from_str("1000000000000uatom").unwrap_test(),
    )));

    let mut node = set_node(
        Some(MockBankKeeper {
            expected_mint_amount: None,
            supply: total_supply.clone(),
        }),
        Some(MockStakingKeeper::new(Decimal256::new(Uint256::from(
            1000000000_u64,
        )))),
    );

    // Well. I simulate chain which runs for month and each block takes 5 seconds
    let mut timestamp = Timestamp::UNIX_EPOCH;
    while timestamp.timestamp_seconds() < TimestampSeconds::try_from(2_628_000).unwrap_test() {
        let _ = node.step(vec![], timestamp);

        timestamp = timestamp
            .checked_add(Duration::new_from_secs(5))
            .unwrap_test();
    }

    if let Some(supply) = &mut *total_supply.acquire_write() {
        supply.amount = supply
            .amount
            .checked_add(Uint256::from(10000000000000000_u64))
            .unwrap_test();
    };

    let _ = node.step(
        vec![],
        timestamp
            .checked_add(Duration::new_from_secs(5))
            .unwrap_test(),
    );

    let q = QueryInflationRequest {};
    let ResponseQuery { value, .. } = node.query(RequestQuery {
        data: q.encode_vec().into(),
        path: QueryInflationRequest::QUERY_URL.to_owned(),
        height: node.height() as i64,
        prove: false,
    });

    let QueryInflationResponse { inflation } =
        QueryInflationResponse::decode_vec(&value).unwrap_test();

    let expected_inflation = Decimal256::from_atomics(2_u8, 1).unwrap_test();

    assert_eq!(expected_inflation, inflation);
}
