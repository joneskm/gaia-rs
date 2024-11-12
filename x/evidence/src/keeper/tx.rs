use super::*;
use crate::{
    errors::{DecodeError, EvidenceAlreadyExistsError, TxEvidenceError},
    types::MsgSubmitEvidence,
};
use gears::{
    context::tx::TxContext,
    tendermint::types::proto::event::{Event, EventAttribute},
};

impl<
        SK: StoreKey,
        StkK: SlashingStakingKeeper<SK, M>,
        SlsK: EvidenceSlashingKeeper<SK, M>,
        E: Evidence + Default,
        M: Module,
    > Keeper<SK, StkK, SlsK, E, M>
where
    <E as std::convert::TryFrom<Any>>::Error: std::fmt::Debug,
{
    /// submit_evidence implements the MsgServer.SubmitEvidence method.
    pub fn submit_evidence_cmd<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        msg: &MsgSubmitEvidence,
    ) -> Result<(), TxEvidenceError> {
        let evidence: E = msg.evidence.clone().try_into().map_err(|_| DecodeError)?;

        self.submit_evidence(ctx, &evidence)?;

        ctx.push_event(Event {
            r#type: "message".to_string(),
            attributes: vec![
                EventAttribute {
                    key: "module".into(),
                    value: "evidence".into(),
                    index: false,
                },
                EventAttribute {
                    key: "sender".into(),
                    value: msg.submitter.to_string().into(),
                    index: false,
                },
            ],
        });

        Ok(())
    }

    /// submit_evidence attempts to match evidence against the keepers evidences and executes
    /// the corresponding registered Evidence handle method and then evidence is
    /// persisted.
    pub fn submit_evidence<DB: Database>(
        &self,
        ctx: &mut TxContext<'_, DB, SK>,
        evidence: &E,
    ) -> Result<(), TxEvidenceError> {
        if self
            .evidence::<TxContext<'_, DB, SK>, DB, E>(ctx, evidence.hash())?
            .is_some()
        {
            return Err(TxEvidenceError::AlreadyExists(EvidenceAlreadyExistsError(
                evidence.hash(),
            )));
        }

        Evidence::handle(ctx, evidence).map_err(|e| TxEvidenceError::Handle(e.to_string()))?;

        ctx.push_event(Event {
            r#type: "submit_evidence".to_string(),
            attributes: vec![EventAttribute {
                key: "evidence_hash".into(),
                value: evidence.hash().to_string().into(),
                index: false,
            }],
        });

        Ok(self.set_evidence(ctx, evidence)?)
    }
}
