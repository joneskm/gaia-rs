const PREFIX_VALIDATOR: &str = "val";
const PREFIX_OPERATOR: &str = "oper";
const PREFIX_CONSENSUS: &str = "cons";

pub(crate) const BECH_32_PREFIX_ACC_ADDR: &str = env!("BECH_32_MAIN_PREFIX");
pub(crate) const BECH_32_PREFIX_VAL_ADDR: &str = constcat::concat!(
    env!("BECH_32_MAIN_PREFIX"),
    PREFIX_VALIDATOR,
    PREFIX_OPERATOR
);
pub(crate) const BECH_32_PREFIX_CONS_ADDR: &str = constcat::concat!(
    env!("BECH_32_MAIN_PREFIX"),
    PREFIX_VALIDATOR,
    PREFIX_CONSENSUS
);

/// Account kind of address
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Account;

/// Validator kind of address
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Validator;

/// Consensus kind of address
#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Consensus;

/// ***Sealed*** trait for address kinds
pub trait AddressKind: sealed::Sealed {
    /// Return prefix for address
    fn prefix() -> &'static str;
}

impl AddressKind for Account {
    fn prefix() -> &'static str {
        BECH_32_PREFIX_ACC_ADDR
    }
}

impl AddressKind for Validator {
    fn prefix() -> &'static str {
        BECH_32_PREFIX_VAL_ADDR
    }
}

impl AddressKind for Consensus {
    fn prefix() -> &'static str {
        BECH_32_PREFIX_CONS_ADDR
    }
}

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl Sealed for Account {}
    impl Sealed for Validator {}
    impl Sealed for Consensus {}
}
