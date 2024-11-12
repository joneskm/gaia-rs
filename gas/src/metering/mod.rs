/// Module for basic gas meter
pub mod basic_meter;
/// Module for config of gas meter.
pub mod config;
/// Module for infinite gas meter.
pub mod infinite_meter;
// Different descriptor for gas meter
pub mod descriptor;
// Kinds of gas meters
pub mod kind;

use std::fmt::Debug;
use std::marker::PhantomData;

use self::kind::MeterKind;

use infinite_meter::InfiniteGasMeter;
use tracing::debug;

pub type FiniteGas = crate::Gas;

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum GasMeteringErrors {
    #[error("out of gas: {0}")]
    ErrorOutOfGas(String),
    #[error("gas overflow: {0}")]
    ErrorGasOverflow(String),
}

#[derive(Debug)]
pub struct ErrorNegativeGasConsumed(pub String);

#[derive(Debug, Clone, Copy)]
pub enum Gas {
    Infinite,
    Finite(FiniteGas),
}

/// This is needed to convert block gas limit from i64 to Gas
impl From<i64> for Gas {
    fn from(val: i64) -> Self {
        // Consistent with Cosmos SDK https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/baseapp/abci.go#L155
        // and https://github.com/cosmos/cosmos-sdk/blob/2582f0aab7b2cbf66ade066fe570a4622cf0b098/baseapp/baseapp.go#L505-L514
        // except that we don't panic if the value < -1 (we just treat it as infinite gas)
        if val > 0 {
            Gas::Finite(val.try_into().expect("val is positive so this won't fail"))
        } else {
            Gas::Infinite
        }
    }
}

impl From<Gas> for i64 {
    fn from(val: Gas) -> i64 {
        match val {
            Gas::Infinite => -1,
            Gas::Finite(val) => val.into(),
        }
    }
}

pub trait PlainGasMeter: Send + Sync + Debug {
    // Return name of this gas meter. Used mainly for debug and logging purposes
    fn name(&self) -> &'static str;
    /// Returns the amount of gas that was consumed by the gas meter instance.
    fn gas_consumed(&self) -> FiniteGas;
    /// Returns the amount of gas that was consumed by gas meter instance, or the limit if it is reached.
    fn gas_consumed_or_limit(&self) -> FiniteGas;
    /// Returns the gas left in the GasMeter.
    fn gas_remaining(&self) -> Gas;
    /// Returns the limit of the gas meter instance.
    fn limit(&self) -> Gas;
    /// Consumes the amount of gas provided.
    /// If the gas overflows, it returns error with the descriptor message.
    /// If the gas meter is not infinite, it returns error  if gas consumed goes above the limit.
    fn consume_gas(&mut self, amount: FiniteGas, descriptor: &str)
        -> Result<(), GasMeteringErrors>;
    // TODO: add refund_gas back in
    /// Deducts the given amount from the gas consumed.
    /// This functionality enables refunding gas to the transaction
    /// or block gas pools so that EVM-compatible chains can fully support the go-ethereum StateDB interface.
    // fn refund_gas(
    //     &mut self,
    //     amount: FiniteGas,
    //     descriptor: &str,
    // ) -> Result<(), ErrorNegativeGasConsumed>;
    /// Returns true if the amount of gas consumed by the gas meter instance is strictly above the limit, false otherwise.
    fn is_past_limit(&self) -> bool;
    /// Returns true if the amount of gas consumed by the gas meter instance is above or equal to the limit, false otherwise.
    fn is_out_of_gas(&self) -> bool;
}

/// Wrapper around any gas meter
#[derive(Debug)]
pub struct GasMeter<DS> {
    meter: Box<dyn PlainGasMeter>,
    _descriptor: PhantomData<DS>,
}

impl<DS> GasMeter<DS> {
    pub fn new(meter: Box<dyn PlainGasMeter>) -> Self {
        Self {
            meter,
            _descriptor: PhantomData,
        }
    }

    pub fn infinite() -> Self {
        Self {
            meter: Box::<InfiniteGasMeter>::default(),
            _descriptor: PhantomData,
        }
    }
}

impl<DS: MeterKind> GasMeter<DS> {
    pub fn replace_meter(&mut self, meter: Box<dyn PlainGasMeter>) {
        let _ = std::mem::replace(&mut self.meter, meter);
    }

    pub fn consumed_or_limit(&self) -> FiniteGas {
        self.meter.gas_consumed_or_limit()
    }

    pub fn consume_gas(
        &mut self,
        amount: FiniteGas,
        descriptor: &str,
    ) -> Result<(), GasMeteringErrors> {
        debug!(
            "Consumed {} gas for {} with {}",
            amount,
            self.meter.name(),
            descriptor
        );
        self.meter.consume_gas(amount, descriptor)
    }

    pub fn is_out_of_gas(&self) -> bool {
        self.meter.is_out_of_gas()
    }

    pub fn limit(&self) -> Gas {
        self.meter.limit()
    }

    pub fn gas_remaining(&self) -> Gas {
        self.meter.gas_remaining()
    }
}
