//! Never failing extension for [Infallible] error

use std::convert::Infallible;

/// Extension method to unwrap infallible errors
pub trait UnwrapInfallible {
    /// Output after unwrap
    type Output;

    /// Unwrap [Infallible] [Result].
    /// This error never could occur, so this is absolutely safe
    fn unwrap_infallible(self) -> Self::Output;
}

impl<T> UnwrapInfallible for Result<T, Infallible> {
    type Output = T;

    fn unwrap_infallible(self) -> Self::Output {
        self.expect("unwrapped infallible. How?")
    }
}
