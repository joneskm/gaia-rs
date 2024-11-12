//! Extension for ergonomic error handling when mapping functions that return [Result]

/// Copy from: [try_map](https://docs.rs/try_map/latest/src/try_map/lib.rs.html#13-19) crate \
/// This method still not a part of std(but have [plans](https://github.com/rust-lang/rfcs/issues/1815) for it) so I copied it temporally
///
/// Extend `Option` with a fallible map method
///
/// This is useful for mapping fallible operations (i.e. operations that)
/// return `Result`, over an optional type. The result will be
/// `Result<Option<U>>`, which makes it easy to handle the errors that
/// originate from inside the closure that's being mapped.
///
/// # Type parameters
///
/// - `T`: The input `Option`'s value type
/// - `U`: The outputs `Option`'s value type
/// - `E`: The possible error during the mapping
pub trait FallibleMapExt<T, U, E> {
    /// Try to apply a fallible map function to the option
    fn try_map<F>(self, f: F) -> Result<Option<U>, E>
    where
        F: FnOnce(T) -> Result<U, E>;
}

impl<T, U, E> FallibleMapExt<T, U, E> for Option<T> {
    fn try_map<F>(self, f: F) -> Result<Option<U>, E>
    where
        F: FnOnce(T) -> Result<U, E>,
    {
        match self {
            Some(x) => f(x).map(Some),
            None => Ok(None),
        }
    }
}
