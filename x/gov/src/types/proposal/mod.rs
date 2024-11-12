use std::{marker::PhantomData, str::FromStr, sync::OnceLock};

use chrono::{DateTime, SubsecRound, Utc};
use gears::{
    core::{errors::CoreError, Protobuf},
    error::ProtobufError,
    gas::store::errors::GasStoreErrors,
    store::database::Database,
    tendermint::types::time::timestamp::Timestamp,
    types::{
        base::coins::UnsignedCoins,
        store::{kv::Store, range::VectoredStoreRange},
        uint::Uint256,
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{errors::SERDE_JSON_CONVERSION, keeper::KEY_PROPOSAL_PREFIX, proposal::Proposal};

pub mod active_iter;
pub mod inactive_iter;

mod inner {
    pub use ibc_proto::cosmos::gov::v1beta1::Proposal;
    pub use ibc_proto::cosmos::gov::v1beta1::TallyResult;
}

// Slight modification of the RFC3339Nano but it right pads all zeros and drops the time zone info
const SORTABLE_DATE_TIME_FORMAT: &str = "%Y-%m-%dT&H:%M:%S.000000000";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalModel<T> {
    pub proposal_id: u64,
    pub content: T,
    pub status: ProposalStatus,
    pub final_tally_result: Option<TallyResult>,
    pub submit_time: Timestamp,
    pub deposit_end_time: Timestamp,
    pub total_deposit: UnsignedCoins,
    pub voting_start_time: Option<Timestamp>,
    pub voting_end_time: Option<Timestamp>,
}

impl<T: Proposal> TryFrom<inner::Proposal> for ProposalModel<T> {
    type Error = ProtobufError;

    fn try_from(
        inner::Proposal {
            proposal_id,
            content,
            status,
            final_tally_result,
            submit_time,
            deposit_end_time,
            total_deposit,
            voting_start_time,
            voting_end_time,
        }: inner::Proposal,
    ) -> Result<Self, Self::Error> {
        let submit_time = submit_time.ok_or(CoreError::MissingField(
            "Proposal: field `submit_time`".to_owned(),
        ))?;

        let deposit_end_time = deposit_end_time.ok_or(CoreError::MissingField(
            "Proposal: field `deposit_end_time`".to_owned(),
        ))?;

        Ok(Self {
            proposal_id,
            content: content
                .ok_or(CoreError::MissingField(
                    "Proposal: field `content`".to_owned(),
                ))?
                .try_into()?,
            status: status.try_into()?,
            final_tally_result: match final_tally_result {
                Some(var) => Some(var.try_into()?),
                None => None,
            },
            submit_time: Timestamp::try_new(submit_time.seconds, submit_time.nanos).map_err(
                |e| CoreError::DecodeGeneral(format!("Proposal: invalid `submit_time`: {e}")),
            )?,
            deposit_end_time: Timestamp::try_new(deposit_end_time.seconds, deposit_end_time.nanos)
                .map_err(|e| {
                    CoreError::DecodeGeneral(format!("Proposal: invalid `deposit_end_time`: {e}"))
                })?,
            total_deposit: UnsignedCoins::new({
                let mut result = Vec::with_capacity(total_deposit.len());

                for coin in total_deposit {
                    result.push(
                        coin.try_into()
                            .map_err(|e| CoreError::Coins(format!("Proposal: {e}")))?,
                    );
                }

                result
            })
            .map_err(|e| CoreError::Coins(e.to_string()))?,
            voting_start_time: match voting_start_time {
                Some(var) => Some(Timestamp::try_new(var.seconds, var.nanos).map_err(|e| {
                    CoreError::DecodeGeneral(format!("Proposal: invalid `voting_start_time`: {e}"))
                })?),
                None => None,
            },
            voting_end_time: match voting_end_time {
                Some(var) => Some(Timestamp::try_new(var.seconds, var.nanos).map_err(|e| {
                    CoreError::DecodeGeneral(format!("Proposal: invalid `voting_end_time`: {e}"))
                })?),
                None => None,
            },
        })
    }
}

impl<T: Proposal> From<ProposalModel<T>> for inner::Proposal {
    fn from(
        ProposalModel {
            proposal_id,
            content,
            status,
            final_tally_result,
            submit_time,
            deposit_end_time,
            total_deposit,
            voting_start_time,
            voting_end_time,
        }: ProposalModel<T>,
    ) -> Self {
        Self {
            proposal_id,
            content: Some(content.into()),
            status: status as i32,
            final_tally_result: final_tally_result.map(|e| e.into()),
            submit_time: Some(ibc_proto::google::protobuf::Timestamp {
                seconds: submit_time.timestamp_seconds().into(),
                nanos: submit_time.nanoseconds().into(),
            }),
            deposit_end_time: Some(ibc_proto::google::protobuf::Timestamp {
                seconds: deposit_end_time.timestamp_seconds().into(),
                nanos: deposit_end_time.nanoseconds().into(),
            }),
            total_deposit: total_deposit.into_iter().map(|this| this.into()).collect(),
            voting_start_time: voting_start_time.map(|this| {
                ibc_proto::google::protobuf::Timestamp {
                    seconds: this.timestamp_seconds().into(),
                    nanos: this.nanoseconds().into(),
                }
            }),
            voting_end_time: voting_end_time.map(|this| ibc_proto::google::protobuf::Timestamp {
                seconds: this.timestamp_seconds().into(),
                nanos: this.nanoseconds().into(),
            }),
        }
    }
}

impl<T: Proposal> Protobuf<inner::Proposal> for ProposalModel<T> {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TallyResult {
    pub yes: Uint256,
    pub abstain: Uint256,
    pub no: Uint256,
    pub no_with_veto: Uint256,
}

impl TryFrom<inner::TallyResult> for TallyResult {
    type Error = CoreError;

    fn try_from(
        inner::TallyResult {
            yes,
            abstain,
            no,
            no_with_veto,
        }: inner::TallyResult,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            yes: Uint256::from_str(&yes)
                .map_err(|e| CoreError::DecodeGeneral(format!("Yes votes parse error: {e}")))?,
            abstain: Uint256::from_str(&abstain)
                .map_err(|e| CoreError::DecodeGeneral(format!("Abstain votes parse error: {e}")))?,
            no: Uint256::from_str(&no)
                .map_err(|e| CoreError::DecodeGeneral(format!("No votes parse error: {e}")))?,
            no_with_veto: Uint256::from_str(&no_with_veto).map_err(|e| {
                CoreError::DecodeGeneral(format!("NoWithVeto votes parse error: {e}"))
            })?,
        })
    }
}

impl From<TallyResult> for inner::TallyResult {
    fn from(
        TallyResult {
            yes,
            abstain,
            no,
            no_with_veto,
        }: TallyResult,
    ) -> Self {
        Self {
            yes: yes.to_string(),
            abstain: abstain.to_string(),
            no: no.to_string(),
            no_with_veto: no_with_veto.to_string(),
        }
    }
}

impl Protobuf<inner::TallyResult> for TallyResult {}

impl<T> ProposalModel<T> {
    const KEY_PREFIX: [u8; 1] = [0x00];
    const KEY_ACTIVE_QUEUE_PREFIX: [u8; 1] = [0x01];
    const KEY_INACTIVE_QUEUE_PREFIX: [u8; 1] = [0x02];

    pub fn key(&self) -> Vec<u8> {
        [
            KEY_PROPOSAL_PREFIX.as_slice(),
            &self.proposal_id.to_be_bytes(),
        ]
        .concat()
    }

    pub fn inactive_queue_key(proposal_id: u64, deposit_end_time: &Timestamp) -> Vec<u8> {
        Self::queue_key(
            &Self::KEY_INACTIVE_QUEUE_PREFIX,
            proposal_id,
            deposit_end_time,
        )
    }

    pub fn active_queue_key(proposal_id: u64, deposit_end_time: &Timestamp) -> Vec<u8> {
        Self::queue_key(
            &Self::KEY_ACTIVE_QUEUE_PREFIX,
            proposal_id,
            deposit_end_time,
        )
    }

    fn queue_key(prefix: &[u8], proposal_id: u64, deposit_end_time: &Timestamp) -> Vec<u8> {
        let date_key = deposit_end_time.format_bytes_rounded();

        [prefix, date_key.as_slice(), &proposal_id.to_be_bytes()].concat()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize, strum::EnumString)]
pub enum ProposalStatus {
    #[strum(serialize = "nil")]
    Nil,
    #[strum(serialize = "deposit")]
    DepositPeriod,
    #[strum(serialize = "voting")]
    VotingPeriod,
    #[strum(serialize = "passed")]
    Passed,
    #[strum(serialize = "rejected")]
    Rejected,
    #[strum(serialize = "failed")]
    Failed,
}

impl TryFrom<i32> for ProposalStatus {
    type Error = CoreError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => ProposalStatus::Nil,
            1 => ProposalStatus::DepositPeriod,
            2 => ProposalStatus::VotingPeriod,
            3 => ProposalStatus::Passed,
            4 => ProposalStatus::Rejected,
            5 => ProposalStatus::Failed,
            _ => Err(CoreError::DecodeGeneral(
                "Proposal status option bigger than possible value".to_owned(),
            ))?,
        })
    }
}

impl From<ProposalStatus> for i32 {
    fn from(value: ProposalStatus) -> Self {
        value as i32
    }
}

fn parse_proposal_key_bytes(bytes: impl AsRef<[u8]>) -> (u64, DateTime<Utc>) {
    static KEY_LENGTH: OnceLock<usize> = OnceLock::new();

    let length_time = *KEY_LENGTH.get_or_init(|| {
        Utc::now()
            .round_subsecs(0)
            .format(SORTABLE_DATE_TIME_FORMAT)
            .to_string()
            .bytes()
            .len()
    });

    let time = DateTime::parse_from_rfc3339(
        core::str::from_utf8(&bytes.as_ref()[1..1 + length_time])
            .expect("We serialize date as String so conversion is save"),
    )
    .unwrap() // TODO
    .to_utc();
    let proposal = u64::from_be_bytes(bytes.as_ref()[1 + length_time..].try_into().unwrap());
    // TODO

    (proposal, time)
}

#[derive(Debug)]
pub struct ProposalsIterator<'a, DB, P>(VectoredStoreRange<'a, DB>, PhantomData<P>);

impl<'a, DB: Database, P: Proposal> ProposalsIterator<'a, DB, P> {
    pub fn new(store: Store<'a, DB>) -> ProposalsIterator<'a, DB, P> {
        let prefix = store.prefix_store(ProposalModel::<P>::KEY_PREFIX);

        let range = prefix.into_range(..);

        ProposalsIterator(range, PhantomData)
    }
}

impl<'a, DB: Database, P: Proposal + DeserializeOwned> Iterator for ProposalsIterator<'a, DB, P> {
    type Item = Result<((u64, DateTime<Utc>), ProposalModel<P>), GasStoreErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(var) = self.0.next() {
            match var {
                Ok((key, value)) => Some(Ok((
                    parse_proposal_key_bytes(key.as_ref()),
                    serde_json::from_slice(&value).expect(SERDE_JSON_CONVERSION),
                ))),
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}
