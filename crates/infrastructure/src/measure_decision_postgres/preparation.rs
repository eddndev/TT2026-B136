use super::{audit, history, inconsistent, port, sources};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_measures::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_measures::MeasureEffect};
use postgres::Transaction;

pub(super) fn supported(command: &MeasureDecisionCommand) -> Result<(), ApplicationError> {
    if command.anchor.is_some() {
        return Err(ApplicationError::InvalidInput(
            "durable anchored measure decisions are not available".into(),
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
    let selections = history::selections(&command.outcome);
    let measure_history = history::candidate(tx, case, &command.outcome, hasher)?;
    let predecessors = history::predecessors(&selections, &measure_history)?;
    for reference in &selections {
        let head: Option<i64> = tx.query_one(
            "SELECT max(revision) FROM case_measure_revisions WHERE case_id=$1 AND measure_id=$2",
            &[&case.as_uuid(), &reference.id().as_uuid()],
        ).map_err(port)?.get(0);
        if head != Some(i64::from(reference.revision().get())) {
            return Err(MeasureDecisionError::SubmissionMismatch.into());
        }
    }
    let mut new_ids = Vec::new();
    for effect in command.outcome.changes().unwrap_or(&[]) {
        match effect {
            MeasureEffect::Impose(proposal) => new_ids.push(proposal.id),
            MeasureEffect::Substitute { successors, .. } => {
                new_ids.extend(successors.iter().map(|proposal| proposal.id));
            }
            _ => {}
        }
    }
    for id in new_ids {
        let exists: bool = tx.query_one(
            "SELECT EXISTS(SELECT 1 FROM case_measures WHERE id=$1)
             OR EXISTS(SELECT 1 FROM case_measure_revisions WHERE measure_id=$1)
             OR EXISTS(SELECT 1 FROM case_measure_decisions WHERE
                outcome_view @> jsonb_build_object('kind','changes','effects',jsonb_build_array(
                    jsonb_build_object('action','impose','proposal',jsonb_build_object('id',$2::text))))
                OR outcome_view @> jsonb_build_object('kind','changes','effects',jsonb_build_array(
                    jsonb_build_object('action','substitute','successors',jsonb_build_array(
                        jsonb_build_object('id',$2::text))))))",
            &[&id.as_uuid(), &id.to_string()],
        ).map_err(port)?.get(0);
        if exists {
            return Err(MeasureDecisionError::OperationConflict.into());
        }
    }
    let support = command.values.support();
    let result_sources = sources::result_sources(tx, case, command, &predecessors, hasher)?;
    let ready = MeasureDecisionReady {
        context,
        support_record: crate::case_stages::documents::load(
            tx,
            case,
            StageSupportRef::new(support.reference(), support.digest()),
            limits,
        )?,
        anchor: None,
        predecessors,
        result_sources,
        measure_history,
    };
    if ready.context.material().case_id != case {
        return Err(inconsistent("current context scope differs"));
    }
    Ok(ready)
}
