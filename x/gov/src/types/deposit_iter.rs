use std::borrow::Cow;

use gears::{
    store::database::Database,
    types::store::{gas::errors::GasStoreErrors, kv::Store, range::StoreRange},
};

use crate::{errors::SERDE_JSON_CONVERSION, msg::deposit::MsgDeposit};

#[derive(Debug)]
pub struct DepositIterator<'a, DB>(StoreRange<'a, DB>);

impl<'a, DB: Database> DepositIterator<'a, DB> {
    pub fn new(store: Store<'a, DB>) -> DepositIterator<'a, DB> {
        let prefix = store.prefix_store(MsgDeposit::KEY_PREFIX);

        // TODO: Unsure that this is correct as golang use prefix to find last key?
        // https://github.com/cosmos/cosmos-sdk/blob/d3f09c222243bb3da3464969f0366330dcb977a8/store/types/utils.go#L10-L12
        let range = prefix.into_range(..);

        DepositIterator(range)
    }
}

impl<'a, DB: Database> Iterator for DepositIterator<'a, DB> {
    type Item = Result<(Cow<'a, Vec<u8>>, MsgDeposit), GasStoreErrors>;

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
