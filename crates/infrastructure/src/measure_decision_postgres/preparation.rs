use super::{anchors, audit, graph::*, history, inconsistent, port, sources, LoadedMeasureHistory};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_measures::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, precautionary_measures::MeasureEffect};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionReady, ApplicationError> {
    load_with_proof(tx, case, command, limits, hasher).map(|(ready, _)| ready)
}
pub(super) fn load_with_proof(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<(MeasureDecisionReady, LoadedMeasureHistory), ApplicationError> {
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
    let mut roots: Vec<_> = selections
        .iter()
        .copied()
        .map(HistoryRoot::Measure)
        .collect();
    let selected_hearing = match command.anchor {
        Some(MeasureDecisionAnchorRef::Precautionary {
            hearing_id,
            revision,
            capture_digest,
        }) => Some(HearingProofRef {
            hearing_id,
            revision,
            capture_digest,
        }),
        _ => None,
    };
    if let Some(reference) = selected_hearing {
        roots.push(HistoryRoot::Hearing(reference));
    }
    let loaded = load_precautionary_history(
        tx,
        case,
        &roots,
        HistoryReserve {
            groups: 1,
            members: command.outcome.affected_ids().len(),
            ..HistoryReserve::default()
        },
        hasher,
    )?;
    let anchor = match selected_hearing {
        Some(reference) => Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
            loaded.hearing_capture(reference)?.clone(),
        ))),
        None => anchors::load(tx, case, &command.anchor, hasher)?,
    };
    let mut refs = selections.clone();
    if let Some(MeasureDecisionAnchorMaterial::Precautionary(capture)) = &anchor {
        refs.extend_from_slice(capture.review.resolved_values.review_targets());
    }
    let measure_history = loaded.subclosure(&refs)?;
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
        anchor,
        predecessors,
        result_sources,
        measure_history,
    };
    if ready.context.material().case_id != case {
        return Err(inconsistent("current context scope differs"));
    }
    Ok((ready, loaded))
}
