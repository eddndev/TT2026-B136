use super::{audit, inconsistent, port, sources, storage, targets};
use crate::measure_decision_postgres::{
    load_precautionary_history, HistoryReserve, HistoryRoot, LoadedMeasureHistory,
};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_hearings::*,
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    hearings::HearingStatus,
    precautionary_hearings::{PrecautionaryHearingId, PrecautionaryHearingRevision},
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingReady, ApplicationError> {
    load_with_proof(tx, case, command, limits, hasher).map(|(ready, _)| ready)
}
pub(super) fn load_with_proof(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<(PrecautionaryHearingReady, LoadedMeasureHistory), ApplicationError> {
    let context = sources::current_context(tx, case, hasher)?;
    let selected = if command.action() == PrecautionaryHearingAction::Schedule {
        if tx
            .query_opt(
                "SELECT id FROM case_precautionary_hearings WHERE id=$1",
                &[&command.hearing_id.as_uuid()],
            )
            .map_err(port)?
            .is_some()
        {
            return Err(PrecautionaryHearingError::OperationConflict.into());
        }
        audit::hearing_absent(tx, command.hearing_id)?;
        None
    } else {
        let reference = storage::selection(tx, case, command.hearing_id, None)?;
        if reference.revision.get() >= 256 {
            return Err(PrecautionaryHearingError::OperationConflict.into());
        }
        Some(reference)
    };
    let fresh_targets = match &command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => values.review_targets(),
        PrecautionaryHearingChange::Cancel { .. } => &[],
    };
    let target_count = if command.action() == PrecautionaryHearingAction::Cancel {
        let reference = selected.ok_or_else(|| inconsistent("cancellation has no current head"))?;
        cancellation_target_count(tx, case, command.hearing_id, reference.revision)?
    } else {
        fresh_targets.len()
    };
    let mut roots: Vec<_> = fresh_targets
        .iter()
        .copied()
        .map(HistoryRoot::Measure)
        .collect();
    if let Some(reference) = selected {
        roots.push(HistoryRoot::Hearing(reference));
    }
    let measures = load_precautionary_history(
        tx,
        case,
        &roots,
        HistoryReserve {
            hearing_captures: 1,
            review_target_occurrences: target_count,
            ..HistoryReserve::default()
        },
        hasher,
    )?;
    let history = selected
        .map(|reference| measures.hearing_operation(reference, hasher))
        .transpose()?
        .map(|stored| stored.history);
    if let Some(previous) = history.as_ref().and_then(|h| h.captures.last()) {
        let expected = match command.change {
            PrecautionaryHearingChange::Replace {
                expected_capture_digest,
                ..
            }
            | PrecautionaryHearingChange::Cancel {
                expected_capture_digest,
                ..
            } => expected_capture_digest,
            PrecautionaryHearingChange::Schedule { .. } => unreachable!(),
        };
        if previous.review.result_revision.get() != command.expected_revision()
            || previous.capture_digest != expected
            || previous.review.status != HearingStatus::Scheduled
        {
            return Err(PrecautionaryHearingError::OperationConflict.into());
        }
    }
    let previous = history.as_ref().and_then(|v| v.captures.last());
    let previous_targets = previous
        .map(|capture| capture.review.resolved_values.review_targets())
        .unwrap_or(&[]);
    let selected_targets = if command.action() == PrecautionaryHearingAction::Cancel {
        previous_targets
    } else {
        fresh_targets
    };
    if selected_targets.len() != target_count {
        return Err(inconsistent(
            "candidate target count differs from scalar reservation",
        ));
    }
    let immediate = targets::union([selected_targets, previous_targets])?;
    let measure_history = measures.subclosure(&immediate)?;
    let selected_sources = match &command.change {
        PrecautionaryHearingChange::Schedule {
            values,
            context: expected,
        }
        | PrecautionaryHearingChange::Replace {
            values,
            context: expected,
            ..
        } => {
            if expected.administration_revision != context.material().administration.revision
                || expected.stage_revision != context.material().stage.stage_revision()
                || expected.context_digest != context.digest(hasher)
            {
                return Err(PrecautionaryHearingError::SubmissionMismatch.into());
            }
            let reference = values.scheduling_basis().support();
            Some(PrecautionaryHearingSelectedSources {
                participants: sources::participants(tx, case, values, previous, true, hasher)?,
                support_record: crate::case_stages::documents::load(
                    tx,
                    case,
                    StageSupportRef::new(reference.reference(), reference.digest()),
                    limits,
                )?,
            })
        }
        PrecautionaryHearingChange::Cancel { .. } => None,
    };
    if context.material().case_id != case {
        return Err(inconsistent("current context scope differs"));
    }
    Ok((
        PrecautionaryHearingReady {
            observed_context: context,
            history,
            selected_sources,
            measure_history,
        },
        measures,
    ))
}

fn cancellation_target_count(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: PrecautionaryHearingId,
    revision: PrecautionaryHearingRevision,
) -> Result<usize, ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT CASE WHEN action IN ('schedule','replace')
                AND jsonb_typeof(values_view->'review_targets')='array'
                THEN jsonb_array_length(values_view->'review_targets') ELSE -1 END AS targets
             FROM case_precautionary_hearing_revisions
             WHERE case_id=$1 AND hearing_id=$2 AND revision<=$3 AND action<>'cancel'
             ORDER BY revision DESC LIMIT 1",
            &[&case.as_uuid(), &id.as_uuid(), &i64::from(revision.get())],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("cancellation has no selected target declaration"))?;
    let count: i32 = row.get("targets");
    if !(0..=32).contains(&count) {
        return Err(inconsistent("cancellation target count exceeds bounds"));
    }
    Ok(count as usize)
}
