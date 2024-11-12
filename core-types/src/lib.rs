//! TODO
#![allow(missing_docs)]

pub mod account;
pub mod any;
pub mod errors;
pub mod query;
pub mod serializers;
pub mod signing;
pub mod tx;

mod types;

pub use crate::types::*;

// Let's call this compatibility
pub use ibc_proto::protobuf::Protobuf;

/// Module for re-exporting things outside of gears. Omit adding anything new here.
pub mod public {
    pub use ibc_proto::protobuf::Protobuf;

    pub mod any {
        pub use crate::any::*;
    }

    pub mod errors {
        pub use crate::errors::*;
    }

    pub mod serializers {
        pub use crate::serializers::*;
    }
}
