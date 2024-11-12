use gears::tendermint::types::proto::event::{Event, EventAttribute};

use super::*;
use crate::{MsgFundCommunityPool, MsgSetWithdrawAddr, MsgWithdrawDelegatorReward};

impl<
        SK: StoreKey,
        PSK: ParamsSubspaceKey,
        AK: AuthKeeper<SK, M>,
        BK: BankKeeper<SK, M>,
        DSK: DistributionStakingKeeper<SK, M>,
        M: Module,
    > Keeper<SK, PSK, AK, BK, DSK, M>
{
    pub fn withdraw_delegator_reward_and_commission<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        msg: &MsgWithdrawDelegatorReward,
    ) -> Result<(), DistributionError> {
        self.withdraw_delegation_rewards(ctx, &msg.delegator_address, &msg.validator_address)?;

        ctx.push_event(Event {
            r#type: "message".to_string(),
            attributes: vec![
                EventAttribute {
                    key: "module".into(),
                    value: self.distribution_module.name().into(),
                    index: false,
                },
                EventAttribute {
                    key: "sender".into(),
                    value: msg.delegator_address.to_string().into(),
                    index: false,
                },
            ],
        });

        if msg.withdraw_commission {
            self.withdraw_validator_commission(ctx, &msg.validator_address)?;
            ctx.push_event(Event {
                r#type: "message".to_string(),
                attributes: vec![
                    EventAttribute {
                        key: "module".into(),
                        value: self.distribution_module.name().into(),
                        index: false,
                    },
                    EventAttribute {
                        key: "sender".into(),
                        value: msg.validator_address.to_string().into(),
                        index: false,
                    },
                ],
            });
        }

        Ok(())
    }

    pub fn set_withdraw_address<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        msg: &MsgSetWithdrawAddr,
    ) -> Result<(), DistributionError> {
        self.set_delegator_withdraw_addr(ctx, &msg.delegator_address, &msg.withdraw_address)?;

        ctx.push_event(Event {
            r#type: "message".to_string(),
            attributes: vec![
                EventAttribute {
                    key: "module".into(),
                    value: self.distribution_module.name().into(),
                    index: false,
                },
                EventAttribute {
                    key: "sender".into(),
                    value: msg.delegator_address.to_string().into(),
                    index: false,
                },
            ],
        });

        Ok(())
    }

    pub fn fund_community_pool_cmd<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        msg: &MsgFundCommunityPool,
    ) -> Result<(), DistributionError> {
        self.fund_community_pool(ctx, msg.amount.clone(), &msg.depositor)?;

        ctx.push_event(Event {
            r#type: "message".to_string(),
            attributes: vec![
                EventAttribute {
                    key: "module".into(),
                    value: self.distribution_module.name().into(),
                    index: false,
                },
                EventAttribute {
                    key: "sender".into(),
                    value: msg.depositor.to_string().into(),
                    index: false,
                },
            ],
        });

        Ok(())
    }
}
