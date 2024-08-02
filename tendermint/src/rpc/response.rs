pub mod block {
    pub use tendermint_rpc::endpoint::block::Response;
}

pub mod tx {
    pub use tendermint_rpc::endpoint::tx::Response;

    pub mod broadcast {
        pub use tendermint_rpc::endpoint::broadcast::tx_commit::Response;
    }

    pub mod search {
        pub use tendermint_rpc::endpoint::tx_search::Response;
    }
}
