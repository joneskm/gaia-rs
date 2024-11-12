use gears::types::{address::ValAddress, decimal256::Decimal256, uint::Uint256};

use crate::msg::weighted_vote::VoteOptionWeighted;

#[derive(Debug)]
pub struct ValidatorGovInfo {
    pub address: ValAddress,
    pub bounded_tokens: Uint256,
    pub delegator_shares: Decimal256,
    pub delegator_deduction: Decimal256,
    pub vote: Vec<VoteOptionWeighted>,
}
