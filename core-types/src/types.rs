pub mod auth {
    pub use ibc_proto::cosmos::tx::v1beta1::AuthInfo;
    pub use ibc_proto::cosmos::tx::v1beta1::Fee;
    pub use ibc_proto::cosmos::tx::v1beta1::Tip;
}

pub mod bank {
    pub use ibc_proto::cosmos::bank::v1beta1::Metadata;
}

pub mod base {
    pub use ibc_proto::cosmos::base::v1beta1::{Coin, IntProto};
}

pub mod msg {
    pub use ibc_proto::cosmos::bank::v1beta1::MsgSend;
}
