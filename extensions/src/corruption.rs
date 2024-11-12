//! Extension for database interaction

use std::fmt::Debug;

/// Message which shows when we unwrap invalid data.
pub const DATABASE_CORRUPTION_MSG: &str = "invalid data in database - possible database corruption";

/// Extension trait for unwrapping
pub trait UnwrapCorrupt {
    /// Result
    type Output;

    /// Unwrap `self`. This method intended to use when you read
    /// data from database which **must** return valid data and
    /// node should be shutdown if something goes wrong.
    fn unwrap_or_corrupt(self) -> Self::Output;
}

impl<T> UnwrapCorrupt for Option<T> {
    type Output = T;

    fn unwrap_or_corrupt(self) -> Self::Output {
        self.expect(DATABASE_CORRUPTION_MSG)
    }
}

impl<T, E: Debug> UnwrapCorrupt for Result<T, E> {
    type Output = T;

    fn unwrap_or_corrupt(self) -> Self::Output {
        self.expect(DATABASE_CORRUPTION_MSG)
    }
}
