use super::handlers::{
    client::{NodeFetcher, QueryHandler, TxHandler},
    AuxHandler,
};
use crate::{
    commands::client::{keys::keys, query::run_query, tx::run_tx, ClientCommands},
    x::query::tx_query::{TxQueryHandler, TxsQueryHandler},
};

/// A Gears client application.
pub trait Client: TxHandler + QueryHandler + AuxHandler {}

#[derive(Debug)]
pub struct ClientApplication<Core: Client, F: NodeFetcher + Clone> {
    core: Core,
    fetcher: F,
}

impl<Core: Client, F: NodeFetcher + Clone> ClientApplication<Core, F> {
    pub fn new(core: Core, fetcher: F) -> Self {
        Self { core, fetcher }
    }

    /// Runs the command passed
    pub fn execute(
        &self,
        command: ClientCommands<Core::AuxCommands, Core::TxCommands, Core::QueryCommands>,
    ) -> anyhow::Result<()> {
        match command {
            ClientCommands::Aux(cmd) => {
                let cmd = self.core.prepare_aux(cmd)?;
                self.core.handle_aux(cmd)?;
            }
            ClientCommands::Tx(cmd) => {
                let tx = run_tx(cmd, &self.core, &self.fetcher)?;

                match tx {
                    crate::commands::client::tx::RuntxResult::Broadcast(tx) => {
                        println!("{}", serde_json::to_string_pretty(&tx)?);
                    }
                    crate::commands::client::tx::RuntxResult::File(file) => {
                        println!("Saved to file: {}", file.to_string_lossy())
                    }
                    crate::commands::client::tx::RuntxResult::None => (),
                }
            }
            ClientCommands::Query(cmd) => {
                let query = run_query(cmd, &self.core)?;

                println!("{}", serde_json::to_string_pretty(&query)?);
            }
            ClientCommands::QueryTx(cmd) => {
                let query = run_query(cmd, &TxQueryHandler::<Core::Message>::new())?;

                println!("{}", serde_json::to_string_pretty(&query)?);
            }
            ClientCommands::QueryTxs(cmd) => {
                let query = run_query(cmd, &TxsQueryHandler::<Core::Message>::new())?;

                println!("{}", serde_json::to_string_pretty(&query)?);
            }
            ClientCommands::Keys(cmd) => keys(cmd)?,
        };

        Ok(())
    }
}
