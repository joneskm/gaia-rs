use std::{cell::RefCell, sync::Arc};

use crate::{
    metering::{config::GasConfig, kind::TxKind, GasMeter, GasMeteringErrors},
    Gas,
};

use super::{
    constants::{
        DELETE_DESC, ITER_NEXT_CAST_FLAT_DESC, READ_COST_FLAT_DESC, READ_PER_BYTE_DESC,
        VALUE_PER_BYTE_DESC, WRITE_COST_FLAT_DESC, WRITE_PER_BYTE_DESC,
    },
    errors::GasStoreErrors,
};

const GUARD_DESC: &str = "GasGuard";

#[derive(Debug, Clone)]
pub struct GasGuard(pub(super) Arc<RefCell<GasMeter<TxKind>>>);

impl GasGuard {
    pub fn new(inner: Arc<RefCell<GasMeter<TxKind>>>) -> Self {
        Self(inner)
    }

    pub fn get(
        &self,
        key: usize,
        value: Option<usize>,
        get_key: &[u8],
    ) -> Result<(), GasStoreErrors> {
        let mut gas_meter = self.0.borrow_mut();

        let read_cost_per_byte = GasConfig::kv().read_cost_per_byte;

        gas_meter
            .consume_gas(GasConfig::kv().read_cost_flat, READ_COST_FLAT_DESC)
            .map_err(|e| GasStoreErrors::new(get_key, e))?;

        gas_meter
            .consume_gas(
                read_cost_per_byte
                    .checked_mul(
                        Gas::try_from(key as u64).map_err(|e| GasStoreErrors::new(get_key, e))?,
                    ) //TODO: should we use the GasStore overflow error once we have one (see TODO below?
                    .ok_or(GasMeteringErrors::ErrorGasOverflow(GUARD_DESC.to_owned())) //TODO: GasStore should define its own overflow error not steal from GasMetering
                    .map_err(|e| GasStoreErrors::new(get_key, e))?,
                READ_PER_BYTE_DESC,
            )
            .map_err(|e| GasStoreErrors::new(get_key, e))?;

        // TODO:NOW is it okay to ignore if value not found. Don't see any other way
        if let Some(value) = value {
            gas_meter
                .consume_gas(
                    read_cost_per_byte
                        .checked_mul(
                            Gas::try_from(value as u64)
                                .map_err(|e| GasStoreErrors::new(get_key, e))?,
                        )
                        .ok_or(GasMeteringErrors::ErrorGasOverflow(GUARD_DESC.to_owned()))
                        .map_err(|e| GasStoreErrors::new(get_key, e))?,
                    READ_PER_BYTE_DESC,
                )
                .map_err(|e| GasStoreErrors::new(get_key, e))?;
        }

        Ok(())
    }

    pub fn set(&self, key: usize, value: usize, set_key: &[u8]) -> Result<(), GasStoreErrors> {
        let mut gas_meter = self.0.borrow_mut();
        gas_meter
            .consume_gas(GasConfig::kv().read_cost_flat, WRITE_COST_FLAT_DESC)
            .map_err(|e| GasStoreErrors::new(set_key, e))?;

        let write_cost_per_byte = GasConfig::kv().write_cost_per_byte;

        gas_meter
            .consume_gas(
                write_cost_per_byte
                    .checked_mul(
                        Gas::try_from(key as u64).map_err(|e| GasStoreErrors::new(set_key, e))?,
                    )
                    .ok_or(GasMeteringErrors::ErrorGasOverflow(GUARD_DESC.to_owned()))
                    .map_err(|e| GasStoreErrors::new(set_key, e))?,
                WRITE_PER_BYTE_DESC,
            )
            .map_err(|e| GasStoreErrors::new(set_key, e))?;

        gas_meter
            .consume_gas(
                write_cost_per_byte
                    .checked_mul(
                        Gas::try_from(value as u64).map_err(|e| GasStoreErrors::new(set_key, e))?,
                    )
                    .ok_or(GasMeteringErrors::ErrorGasOverflow(GUARD_DESC.to_owned()))
                    .map_err(|e| GasStoreErrors::new(set_key, e))?,
                WRITE_PER_BYTE_DESC,
            )
            .map_err(|e| GasStoreErrors::new(set_key, e))?;

        Ok(())
    }

    pub fn delete(&self, delete_key: &[u8]) -> Result<(), GasStoreErrors> {
        self.0
            .borrow_mut()
            .consume_gas(GasConfig::kv().delete_cost, DELETE_DESC)
            .map_err(|e| GasStoreErrors::new(delete_key, e))?;

        Ok(())
    }

    pub fn range(&self, key_value: Option<(usize, usize, &[u8])>) -> Result<(), GasStoreErrors> {
        let mut gas_meter = self.0.borrow_mut();

        if let Some((key, value, get_key)) = key_value {
            let read_cost_per_byte = GasConfig::kv().read_cost_per_byte;

            gas_meter
                .consume_gas(
                    read_cost_per_byte
                        .checked_mul(
                            Gas::try_from(key as u64)
                                .map_err(|e| GasStoreErrors::new(get_key, e))?,
                        )
                        .ok_or(GasMeteringErrors::ErrorGasOverflow(GUARD_DESC.to_owned()))
                        .map_err(|e| GasStoreErrors::new(get_key, e))?,
                    VALUE_PER_BYTE_DESC,
                )
                .map_err(|e| GasStoreErrors::new(get_key, e))?;

            gas_meter
                .consume_gas(
                    read_cost_per_byte
                        .checked_mul(
                            Gas::try_from(value as u64)
                                .map_err(|e| GasStoreErrors::new(get_key, e))?,
                        )
                        .ok_or(GasMeteringErrors::ErrorGasOverflow(GUARD_DESC.to_owned()))
                        .map_err(|e| GasStoreErrors::new(get_key, e))?,
                    VALUE_PER_BYTE_DESC,
                )
                .map_err(|e| GasStoreErrors::new(get_key, e))?;

            gas_meter
                .consume_gas(
                    GasConfig::kv().iter_next_cost_flat,
                    ITER_NEXT_CAST_FLAT_DESC,
                )
                .map_err(|e| GasStoreErrors::new(get_key, e))?; // I'm unsure how to handle such case
        }

        Ok(())
    }
}
