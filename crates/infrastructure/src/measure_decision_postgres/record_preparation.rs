use super::{
    anchors, audit, graph::*, history, inconsistent, port, record_sources, sources,
    LoadedMeasureHistory,
};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_measures::*,
    ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureEffect,
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<MeasureDecisionRecordReady, ApplicationError> {
    load_with_proof(tx, case, command, limits, hasher).map(|(ready, _)| ready)
}

pub(super) fn load_with_proof(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &MeasureDecisionCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<(MeasureDecisionRecordReady, LoadedMeasureHistory), ApplicationError> {
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
    let record_history = loaded.record_subclosure(&refs)?;
    let predecessors = record_sources::predecessors(hasher, case, &selections, &record_history)?;
    current_heads(tx, case, &selections)?;
    fresh_identities(tx, command)?;
    let support = command.values.support();
    let result_sources = record_sources::result_sources(tx, case, command, &predecessors, hasher)?;
    let ready = MeasureDecisionRecordReady {
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
        record_history,
    };
    if ready.context.material().case_id != case {
        return Err(inconsistent("current context scope differs"));
    }
    Ok((ready, loaded))
}

fn current_heads(
    tx: &mut Transaction<'_>,
    case: CaseId,
    selections: &[PrecautionaryMeasureRef],
) -> Result<(), ApplicationError> {
    for reference in selections {
        let head = tx
            .query_opt(
                "SELECT revision,validity,family FROM case_measure_revisions
            WHERE case_id=$1 AND measure_id=$2 ORDER BY revision DESC LIMIT 1",
                &[&case.as_uuid(), &reference.id().as_uuid()],
            )
            .map_err(port)?
            .ok_or(MeasureDecisionError::NotFound)?;
        if head.get::<_, i64>("revision") != i64::from(reference.revision().get()) {
            return Err(MeasureDecisionError::SubmissionMismatch.into());
        }
        if head.get::<_, String>("validity") != "valid"
            || !matches!(head.get::<_, String>("family").as_str(), "m1" | "m2" | "c1")
        {
            return Err(inconsistent(
                "current predecessor family or captured validity is ineligible",
            ));
        }
    }
    Ok(())
}

fn fresh_identities(
    tx: &mut Transaction<'_>,
    command: &MeasureDecisionCommand,
) -> Result<(), ApplicationError> {
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
    Ok(())
}
