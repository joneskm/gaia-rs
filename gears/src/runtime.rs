use std::sync::OnceLock;

use tokio::runtime::Runtime;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Access single runtime for all async methods which comes from 3rd party crates
///
/// **WARNING**: never use async runtime within async context.
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("failed to create tokio runtime"))
}
