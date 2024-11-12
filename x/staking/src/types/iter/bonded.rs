use crate::{consts::keeper::VALIDATORS_KEY, Validator};
use gears::{
    core::Protobuf, extensions::corruption::UnwrapCorrupt, gas::store::errors::GasStoreErrors,
    store::database::Database, types::store::kv::Store, x::types::validator::BondStatus,
};

#[derive(Debug)]
pub struct BondedValidatorsIterator {
    inner: Vec<Result<Validator, GasStoreErrors>>, // TODO: we missing double ended iterator implementation currently so instead load all validators and read from end...
    position: usize,
    max_validator: u32,
}

impl BondedValidatorsIterator {
    pub fn new<DB: Database>(store: Store<'_, DB>, max_validator: u32) -> BondedValidatorsIterator {
        BondedValidatorsIterator {
            inner: store
                .prefix_store(VALIDATORS_KEY)
                .into_range(..)
                .filter_map(|this| {
                    let mut is_bonded = false;
                    let v = this.map(|(_, value)| {
                        let validator = Validator::decode_vec(&value).unwrap_or_corrupt();
                        is_bonded = validator.status == BondStatus::Bonded;
                        validator
                    });
                    if is_bonded {
                        Some(v)
                    } else {
                        None
                    }
                })
                .collect(),
            position: 0,
            max_validator,
        }
    }
}

impl Iterator for BondedValidatorsIterator {
    type Item = Result<Validator, GasStoreErrors>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.max_validator as usize {
            return None; // TODO:NOW Option or Error?
        }

        if let Some(var) = self.inner.pop() {
            match var {
                Ok(validator) => {
                    if validator.status == BondStatus::Bonded {
                        self.position += 1;
                        Some(Ok(validator))
                    } else {
                        self.next()
                    }
                }
                Err(err) => Some(Err(err)),
            }
        } else {
            None
        }
    }
}
