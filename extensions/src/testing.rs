//! Extension for testing

use std::fmt::Debug;

/// Message when you unwrap
pub const TESTING_MSG: &str = "unwrap value in test";

/// Extension method to unwrap during tests
pub trait UnwrapTesting {
    /// Output after unwrap
    type Output;

    /// Unwrap to get `Output`. Do this only in test code in case you
    /// have forbid direct `unwrap` or don't want to see it during search
    fn unwrap_test(self) -> Self::Output;
}

impl<T> UnwrapTesting for Option<T> {
    type Output = T;

    fn unwrap_test(self) -> Self::Output {
        self.expect(TESTING_MSG)
    }
}

impl<T, E: Debug> UnwrapTesting for Result<T, E> {
    type Output = T;

    fn unwrap_test(self) -> Self::Output {
        self.expect(TESTING_MSG)
    }
}
