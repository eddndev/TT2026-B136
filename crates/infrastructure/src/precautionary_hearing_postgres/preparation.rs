use super::{audit, inconsistent, port, sources, storage, targets};
use crate::measure_decision_postgres::load_measure_targets;
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_hearings::*,
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, hearings::HearingStatus};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingReady, ApplicationError> {
    let context = sources::current_context(tx, case, hasher)?;
    let prefix = if command.action() == PrecautionaryHearingAction::Schedule {
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
        let prefix = storage::prefix(tx, case, command.hearing_id, None)?;
        if prefix.len() >= 256 {
            return Err(PrecautionaryHearingError::OperationConflict.into());
        }
        Some(prefix)
    };
    let previous_targets = prefix.as_ref().map(|p| p.last_targets()).unwrap_or(&[]);
    let selected_targets = match &command.change {
        PrecautionaryHearingChange::Schedule { values, .. }
        | PrecautionaryHearingChange::Replace { values, .. } => values.review_targets(),
        PrecautionaryHearingChange::Cancel { .. } => previous_targets,
    };
    let immediate = targets::union([selected_targets, previous_targets])?;
    let refs = targets::union([
        immediate.as_slice(),
        prefix.as_ref().map(|p| p.refs.as_slice()).unwrap_or(&[]),
    ])?;
    let measures = load_measure_targets(tx, case, &refs, hasher)?;
    let history = prefix
        .as_ref()
        .map(|prefix| storage::reconstruct(tx, prefix, &measures, hasher))
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
    let measure_history = measures.subclosure(&immediate)?;
    let previous = history.as_ref().and_then(|v| v.captures.last());
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
    Ok(PrecautionaryHearingReady {
        observed_context: context,
        history,
        selected_sources,
        measure_history,
    })
}
