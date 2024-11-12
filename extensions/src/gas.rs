//! Extension for gas related stuff

use std::fmt::Debug;

/// Message when you unwrap
pub const NO_GAS_IN_CTX: &str = "Context shouldn't have any gas so it's safe to unwrap";

/// Extension trait for gas unwrapping
pub trait GasResultExt {
    /// Output after unwrap
    type Output;

    /// Unwrap result with gas.
    ///
    /// This method exists only due single api for all contexts.
    /// If you know that method will be used only in context without gas
    /// use infallible variants.
    ///
    /// **Note** never use this method when using `TxContext` or any other
    /// which contain gas metering. Method intended to reduce boilerplate
    /// for queries or {begin/end}_block.
    fn unwrap_gas(self) -> Self::Output;
}

/// Bound which gas error should implement
pub trait UnwrapGasError: Debug {}

impl<T, U: UnwrapGasError> GasResultExt for Result<T, U> {
    type Output = T;

    fn unwrap_gas(self) -> Self::Output {
        self.expect(NO_GAS_IN_CTX)
    }
}
