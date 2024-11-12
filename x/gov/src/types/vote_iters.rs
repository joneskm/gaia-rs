use std::borrow::Cow;

use gears::{
    gas::store::errors::GasStoreErrors,
    store::database::Database,
    types::store::{kv::Store, range::VectoredStoreRange},
};

use crate::{errors::SERDE_JSON_CONVERSION, msg::weighted_vote::MsgVoteWeighted};

#[derive(Debug)]
pub struct WeightedVoteIterator<'a, DB>(VectoredStoreRange<'a, DB>);

impl<'a, DB: Database> WeightedVoteIterator<'a, DB> {
    pub fn new(store: Store<'a, DB>, proposal_id: u64) -> WeightedVoteIterator<'a, DB> {
        let prefix = store.prefix_store(
            [
                MsgVoteWeighted::KEY_PREFIX.to_vec(),
                proposal_id.to_be_bytes().to_vec(),
            ]
            .concat(),
        );

        let range = prefix.into_range(..);

        WeightedVoteIterator(range)
    }
}

impl<'a, DB: Database> Iterator for WeightedVoteIterator<'a, DB> {
    type Item = Result<(Cow<'a, Vec<u8>>, MsgVoteWeighted), GasStoreErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(var) = self.0.next() {
            match var {
                Ok((key, value)) => Some(Ok((
                    key,
                    serde_json::from_slice(&value).expect(SERDE_JSON_CONVERSION),
                ))),
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}
