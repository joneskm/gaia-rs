use std::{marker::PhantomData, ops::Bound};

use chrono::{DateTime, Utc};
use gears::{
    gas::store::errors::GasStoreErrors,
    store::database::Database,
    tendermint::types::time::timestamp::Timestamp,
    types::store::{kv::Store, range::VectoredStoreRange},
};
use serde::de::DeserializeOwned;

use crate::{errors::SERDE_JSON_CONVERSION, proposal::Proposal};

use super::{parse_proposal_key_bytes, ProposalModel};

#[derive(Debug)]
pub struct ActiveProposalIterator<'a, DB, P>(VectoredStoreRange<'a, DB>, PhantomData<P>);

impl<'a, DB: Database, P: Proposal> ActiveProposalIterator<'a, DB, P> {
    pub fn new(store: Store<'a, DB>, end_time: &Timestamp) -> ActiveProposalIterator<'a, DB, P> {
        Self(
            store.into_range((
                Bound::Included(ProposalModel::<P>::KEY_ACTIVE_QUEUE_PREFIX.to_vec()),
                Bound::Excluded(
                    [
                        ProposalModel::<P>::KEY_ACTIVE_QUEUE_PREFIX.as_slice(),
                        end_time.format_bytes_rounded().as_slice(),
                    ]
                    .concat()
                    .to_vec(),
                ),
            )),
            PhantomData,
        )
    }
}

impl<'a, DB: Database, P: Proposal + DeserializeOwned> Iterator
    for ActiveProposalIterator<'a, DB, P>
{
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
