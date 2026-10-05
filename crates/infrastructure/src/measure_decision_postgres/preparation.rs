use super::{audit, inconsistent, port, sources};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_measures::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_measures::MeasureEffect};
use postgres::Transaction;

pub(super) fn supported(command: &MeasureDecisionCommand) -> Result<(), ApplicationError> {
    if command.anchor.is_some()
        || command.outcome.changes().is_some_and(|effects| {
            effects
                .iter()
                .any(|effect| !matches!(effect, MeasureEffect::Impose(_)))
        })
    {
        return Err(ApplicationError::InvalidInput(
            "durable anchored and predecessor measure decisions are not available".into(),
        ));
    }
    Ok(())
}
pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionReady, ApplicationError> {
    supported(command)?;
    audit::inventory_intact(tx)?;
    let context = sources::current_context(tx, case, hasher)?;
    if command.context.administration_revision != context.material().administration.revision
        || command.context.stage_revision != context.material().stage.stage_revision()
        || command.context.context_digest != context.digest(hasher)
    {
        return Err(MeasureDecisionError::SubmissionMismatch.into());
    }
    let exists: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM case_measure_decisions WHERE decision_id=$1)",
            &[&command.decision_id.as_uuid()],
        )
        .map_err(port)?
        .get(0);
    if exists {
        return Err(MeasureDecisionError::OperationConflict.into());
    }
    audit::decision_absent(tx, command.decision_id)?;
    for id in command.outcome.affected_ids() {
        let exists:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM case_measures WHERE id=$1) OR EXISTS(SELECT 1 FROM case_measure_revisions WHERE measure_id=$1) OR EXISTS(SELECT 1 FROM case_measure_decisions WHERE outcome_view @> jsonb_build_object('kind','changes','effects',jsonb_build_array(jsonb_build_object('action','impose','proposal',jsonb_build_object('id',$2::text)))))",&[&id.as_uuid(),&id.to_string()]).map_err(port)?.get(0);
        if exists {
            return Err(MeasureDecisionError::OperationConflict.into());
        }
    }
    let support = command.values.support();
    let ready = MeasureDecisionReady {
        context,
        support_record: crate::case_stages::documents::load(
            tx,
            case,
            StageSupportRef::new(support.reference(), support.digest()),
            limits,
        )?,
        anchor: None,
        predecessors: vec![],
        result_sources: sources::result_sources(tx, case, command, hasher)?,
        measure_history: MeasureHistoryEvidence { groups: vec![] },
    };
    if ready.context.material().case_id != case {
        return Err(inconsistent("current context scope differs"));
    }
    Ok(ready)
}
