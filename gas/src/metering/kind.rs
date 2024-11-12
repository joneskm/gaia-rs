pub trait MeterKind: sealed::Sealed {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BlockKind;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxKind;

impl MeterKind for BlockKind {}
impl MeterKind for TxKind {}

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl Sealed for BlockKind {}
    impl Sealed for TxKind {}
}
