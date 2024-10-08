pub mod client;
pub mod abci_handler;
mod handler;
pub mod keeper;
pub mod types;

pub use crate::handler::*;

pub trait Module:
    Clone + Send + Sync + TryFrom<Vec<u8>> + std::cmp::Eq + std::hash::Hash + 'static
{
    fn name(&self) -> &'static str;
}
