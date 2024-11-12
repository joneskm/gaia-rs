use auth::cli::query::{AuthQuery, AuthQueryResponse};
use bank::cli::query::{BankQuery, BankQueryResponse};
use gears::{baseapp::Query, derive::Query};
use ibc_rs::client::cli::query::{IbcQuery, IbcQueryResponse};
use serde::{Deserialize, Serialize};
use staking::cli::query::{StakingQuery, StakingQueryResponse};

#[derive(Clone, Debug, PartialEq)]
pub enum GaiaQuery {
    Auth(AuthQuery),
    Bank(BankQuery),
    Staking(StakingQuery),
    Ibc(IbcQuery),
}

impl Query for GaiaQuery {
    fn query_url(&self) -> &'static str {
        match self {
            GaiaQuery::Auth(var) => var.query_url(),
            GaiaQuery::Bank(var) => var.query_url(),
            GaiaQuery::Staking(var) => var.query_url(),
            GaiaQuery::Ibc(var) => var.query_url(),
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        match self {
            GaiaQuery::Auth(var) => var.into_bytes(),
            GaiaQuery::Bank(var) => var.into_bytes(),
            GaiaQuery::Staking(var) => var.into_bytes(),
            GaiaQuery::Ibc(var) => var.into_bytes(),
        }
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug, Query)]
#[serde(untagged)]
pub enum GaiaQueryResponse {
    Auth(AuthQueryResponse),
    Bank(BankQueryResponse),
    Staking(StakingQueryResponse),
    Ibc(IbcQueryResponse),
}
