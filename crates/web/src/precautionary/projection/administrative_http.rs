use super::{administrative, history};
use crate::error::ApiError;
use application::measure_corrections::*;
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryMeasureRef,
    precautionary_measures::MeasureCorrectionOperationId,
};
use serde_json::{json, Value};

pub(crate) fn admin_review(
    value: &MeasureAdministrativeReview,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    review_shape(value)?;
    administrative::review(value, hasher)
}

pub(crate) fn admin_operation(
    value: &MeasureAdministrativeStoredOperation,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    bound_admin(value, value.capture.review.case_id, None)?;
    Ok(
        json!({"capture":administrative::capture(&value.capture,hasher)?,
        "origin":administrative::origin(&value.origin),
        "record_history":history::project(&value.record_history,hasher)?}),
    )
}

pub(crate) fn bound_admin(
    value: &MeasureAdministrativeStoredOperation,
    case: CaseId,
    operation: Option<MeasureCorrectionOperationId>,
) -> Result<(), ApiError> {
    history::check(&value.record_history)?;
    if value.capture.records.len() > 2 {
        return Err(ApiError::internal());
    }
    let evidence = &value.record_history;
    let owners = evidence.records.judicial.groups.len()
        + evidence.records.administrative.len()
        + evidence.decisions.len();
    let members = evidence
        .records
        .judicial
        .groups
        .iter()
        .map(|group| group.capture.measures.len())
        .sum::<usize>()
        + evidence
            .decisions
            .iter()
            .map(|group| group.capture.measures.len())
            .sum::<usize>()
        + evidence
            .records
            .administrative
            .iter()
            .map(|group| group.capture.records.len())
            .sum::<usize>();
    if owners >= 256
        || members + value.capture.records.len() > 8192
        || operation.is_some_and(|id| id != value.capture.review.command.operation_id)
    {
        return Err(ApiError::internal());
    }
    bound_capture(&value.capture, &value.origin, case)
}

pub(super) fn bound_capture(
    capture: &MeasureAdministrativeCapture,
    origin: &MeasureAdministrativeOrigin,
    case: CaseId,
) -> Result<(), ApiError> {
    let review = &capture.review;
    review_shape(review)?;
    let expected_rows = if review.replacement.is_some() { 2 } else { 1 };
    if review.case_id != case
        || origin.case_id != case
        || origin.operation_id != review.command.operation_id
        || origin.submission_digest != review.submission_digest
        || origin.review_digest != review.review_digest
        || origin.capture_digest != capture.capture_digest
        || capture.records.len() != expected_rows
        || capture.replacement_link.is_some() != review.replacement.is_some()
        || capture.records.windows(2).any(|pair| {
            (pair[0].result.id.as_uuid(), pair[0].result.revision)
                >= (pair[1].result.id.as_uuid(), pair[1].result.revision)
        })
    {
        return Err(ApiError::internal());
    }
    for row in &capture.records {
        if row.case_id != case
            || row.operation_id != review.command.operation_id
            || row.review_digest != review.review_digest
            || row.actor != review.actor
            || row.context != review.context
            || row.support != review.support
            || row.recorded_at != capture.recorded_at
            || (row.result != review.result && Some(&row.result) != review.replacement.as_ref())
        {
            return Err(ApiError::internal());
        }
    }
    let original = capture
        .records
        .iter()
        .find(|row| row.result == review.result)
        .ok_or_else(ApiError::internal)?;
    if let (Some(replacement), Some(link)) = (&review.replacement, &capture.replacement_link) {
        let new = capture
            .records
            .iter()
            .find(|row| &row.result == replacement)
            .ok_or_else(ApiError::internal)?;
        if link.entered_in_error != reference(original) || link.replacement != reference(new) {
            return Err(ApiError::internal());
        }
    }
    Ok(())
}

fn review_shape(value: &MeasureAdministrativeReview) -> Result<(), ApiError> {
    let result = &value.result;
    let command = &value.command;
    if value.context.material().case_id != value.case_id
        || result.id != command.target.id()
        || result.previous != command.target
        || command.target.revision().get().checked_add(1) != Some(result.revision.get())
    {
        return Err(ApiError::internal());
    }
    match &command.action {
        MeasureAdministrativeAction::Correct(_) => {
            if value.replacement.is_some() || result.validity != MeasureCaptureValidity::Valid {
                return Err(ApiError::internal());
            }
        }
        MeasureAdministrativeAction::MarkEnteredInError => {
            if value.replacement.is_some()
                || result.validity != MeasureCaptureValidity::EnteredInError
            {
                return Err(ApiError::internal());
            }
        }
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id,
            subject,
        } => {
            let new = value.replacement.as_ref().ok_or_else(ApiError::internal)?;
            if result.validity != MeasureCaptureValidity::EnteredInError
                || *replacement_id == result.id
                || new.id != *replacement_id
                || new.revision.get() != 1
                || new.previous != command.target
                || new.validity != MeasureCaptureValidity::Valid
                || new.values.subject() != *subject
                || new.record_root
                    != (MeasureRecordRoot::Administrative {
                        operation_id: command.operation_id,
                        measure_id: *replacement_id,
                    })
            {
                return Err(ApiError::internal());
            }
        }
    }
    Ok(())
}

pub(super) fn reference(row: &MeasureAdministrativeRecordCapture) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(row.result.id, row.result.revision, row.capture_digest)
}
