use std::fmt::Display;

use super::{FiniteGas, Gas, GasMeteringErrors, PlainGasMeter};

/// Gas meter without consumption limit
#[derive(Debug, Clone)]
pub struct InfiniteGasMeter {
    consumed: FiniteGas,
}

impl Default for InfiniteGasMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl InfiniteGasMeter {
    /// Create new `InfiniteGasMeter` with zero consumed gas.
    pub const fn new() -> Self {
        Self {
            consumed: FiniteGas::ZERO,
        }
    }
}

impl PlainGasMeter for InfiniteGasMeter {
    fn gas_consumed(&self) -> FiniteGas {
        self.consumed
    }

    fn gas_consumed_or_limit(&self) -> FiniteGas {
        self.consumed
    }

    fn gas_remaining(&self) -> Gas {
        Gas::Infinite
    }

    fn limit(&self) -> Gas {
        Gas::Infinite
    }

    fn consume_gas(
        &mut self,
        amount: FiniteGas,
        descriptor: &str,
    ) -> Result<(), GasMeteringErrors> {
        if let Some(sum) = self.consumed.checked_add(amount) {
            self.consumed = sum;
            Ok(())
        } else {
            Err(GasMeteringErrors::ErrorGasOverflow(descriptor.to_owned()))
        }
    }

    // fn refund_gas(
    //     &mut self,
    //     amount: FiniteGas,
    //     descriptor: &str,
    // ) -> Result<(), ErrorNegativeGasConsumed> {
    //     if self.consumed < amount {
    //         Err(ErrorNegativeGasConsumed(descriptor.to_owned()))
    //     } else {
    //         self.consumed.0 -= amount.0;

    //         Ok(())
    //     }
    // }

    fn is_past_limit(&self) -> bool {
        false
    }

    fn is_out_of_gas(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "gears infinite meter"
    }
}

impl Display for InfiniteGasMeter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InfiniteGasMeter: consumed {}", self.consumed)
    }
}
