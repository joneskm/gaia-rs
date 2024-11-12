//! Extension for locking

use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

const POISONED_LOCK_MSG: &str = "poisoned lock";

/// Extension to reduce locking boilerplate
pub trait AcquireRwLock {
    /// Output after unwrap
    type Output;

    /// Acquire read lock. Unwraps locking result.
    /// If you want handle it clear lock before calling or use `std` methods
    fn acquire_read(&self) -> RwLockReadGuard<'_, Self::Output>;
    /// Acquire write lock. Unwraps locking result.
    /// If you want handle it clear lock before calling or use `std` methods
    fn acquire_write(&self) -> RwLockWriteGuard<'_, Self::Output>;
}

impl<T> AcquireRwLock for RwLock<T> {
    type Output = T;

    fn acquire_read(&self) -> RwLockReadGuard<'_, Self::Output> {
        self.read().expect(POISONED_LOCK_MSG)
    }

    fn acquire_write(&self) -> RwLockWriteGuard<'_, Self::Output> {
        self.write().expect(POISONED_LOCK_MSG)
    }
}
