use std::sync::OnceLock;

use super::FiniteGas;

#[derive(Debug)]
pub struct GasConfig {
    pub has_cost: FiniteGas,
    pub delete_cost: FiniteGas,
    pub read_cost_flat: FiniteGas,
    pub read_cost_per_byte: FiniteGas,
    pub write_cost_flat: FiniteGas,
    pub write_cost_per_byte: FiniteGas,
    pub iter_next_cost_flat: FiniteGas,
}

impl GasConfig {
    pub fn kv() -> &'static Self {
        static DEFAULT_KV_CONFIG: OnceLock<GasConfig> = OnceLock::new();

        DEFAULT_KV_CONFIG.get_or_init(|| Self {
            has_cost: FiniteGas::from(1000_u16),
            delete_cost: FiniteGas::from(1000_u32),
            read_cost_flat: FiniteGas::from(1000_u32),
            read_cost_per_byte: FiniteGas::from(3_u8),
            write_cost_flat: FiniteGas::from(2000_u32),
            write_cost_per_byte: FiniteGas::from(30_u8),
            iter_next_cost_flat: FiniteGas::from(30_u8),
        })
    }

    pub fn default_transient() -> &'static Self {
        static DEFAULT_TRANSIENT_CONFIG: OnceLock<GasConfig> = OnceLock::new();

        DEFAULT_TRANSIENT_CONFIG.get_or_init(|| Self {
            has_cost: FiniteGas::from(1000_u16),
            delete_cost: FiniteGas::from(100_u8),
            read_cost_flat: FiniteGas::from(100_u8),
            read_cost_per_byte: FiniteGas::from(0_u8),
            write_cost_flat: FiniteGas::from(200_u8),
            write_cost_per_byte: FiniteGas::from(3_u8),
            iter_next_cost_flat: FiniteGas::from(3_u8),
        })
    }
}
