use std::borrow::Cow;

use gears::{
    context::QueryableContext,
    core::Protobuf,
    extensions::corruption::UnwrapCorrupt,
    gas::store::errors::GasStoreErrors,
    store::{
        database::{prefix::PrefixDB, Database},
        StoreKey,
    },
    types::{address::ValAddress, store::range::VectoredStoreRange},
};

use crate::{keys::validator_slash_event_key_prefix, ValidatorSlashEvent};

#[derive(Debug)]
pub struct SlashEventIterator<'a, DB> {
    ranges: Vec<VectoredStoreRange<'a, PrefixDB<DB>>>,
    current: usize,
}

impl<'a, DB: Database> SlashEventIterator<'a, DB> {
    pub fn new<CTX: QueryableContext<DB, SK>, SK: StoreKey>(
        ctx: &'a CTX,
        sk: &SK,
        val_address: &ValAddress,
        starting_height: u64,
        ending_height: u64,
    ) -> SlashEventIterator<'a, DB> {
        let mut ranges = vec![];
        for height in starting_height..ending_height + 1 {
            let store = ctx.kv_store(sk);
            let key = validator_slash_event_key_prefix(val_address.clone(), height);
            ranges.push(store.prefix_store(key).into_range(..));
        }

        SlashEventIterator { ranges, current: 0 }
    }
}

impl<'a, DB: Database> Iterator for SlashEventIterator<'a, DB> {
    type Item = Result<(Cow<'a, Vec<u8>>, ValidatorSlashEvent), GasStoreErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(var) = self.ranges[self.current].next() {
            match var {
                Ok((key, value)) => Some(Ok((
                    key,
                    ValidatorSlashEvent::decode_vec(&value).unwrap_or_corrupt(),
                ))),
                Err(err) => Some(Err(err)),
            }
        } else if self.current <= self.ranges.len() {
            self.current += 1;
            self.next()
        } else {
            None
        }
    }
}
