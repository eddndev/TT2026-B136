use super::{audit, inconsistent, port, sources, storage};
use application::{
    case_stages::StageSupportRef, documents::StageSupportReadLimits, precautionary_hearings::*,
    precautionary_measures::MeasureHistoryEvidence, ApplicationError,
};
use domain::{
    cases::CaseId, crypto::DocumentHasher, hearings::HearingStatus,
    precautionary_hearings::PrecautionaryHearingPurpose,
};
use postgres::Transaction;

pub(super) fn load(
    tx: &mut Transaction<'_>,
    case: CaseId,
    command: &PrecautionaryHearingCommand,
    limits: &StageSupportReadLimits,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingReady, ApplicationError> {
    let context = sources::current_context(tx, case, hasher)?;
    let history = if command.action() == PrecautionaryHearingAction::Schedule {
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
        let previous = storage::detail(tx, case, command.hearing_id, None, hasher)?;
        let expected = match command.change {
            PrecautionaryHearingChange::Replace {
                expected_capture_digest,
                ..
            }
            | PrecautionaryHearingChange::Cancel {
                expected_capture_digest,
                ..
            } => expected_capture_digest,
            _ => unreachable!(),
        };
        if previous.capture.review.result_revision.get() != command.expected_revision()
            || previous.capture.capture_digest != expected
            || previous.capture.review.status != HearingStatus::Scheduled
            || previous.history.captures.len() >= 256
        {
            return Err(PrecautionaryHearingError::OperationConflict.into());
        }
        Some(previous.history)
    };
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
            if values.purpose() != PrecautionaryHearingPurpose::Imposition {
                return Err(ApplicationError::InvalidInput(
                    "review hearing requires durable measure history".into(),
                ));
            }
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
        measure_history: MeasureHistoryEvidence { groups: vec![] },
    })
}
