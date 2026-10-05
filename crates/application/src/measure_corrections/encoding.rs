use super::{wire::*, *};
use crate::{identity::Principal, ApplicationError};
use domain::cases::CaseId;

pub fn measure_administrative_submission_bytes(
    principal: &Principal,
    case_id: CaseId,
    command: &MeasureAdministrativeCommand,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"MATXN1".to_vec();
    actor(&mut bytes, principal)?;
    bytes.extend_from_slice(case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(command.operation_id.as_uuid().as_bytes());
    reference(&mut bytes, command.target);
    bytes.extend_from_slice(&command.context.administration_revision.get().to_be_bytes());
    bytes.extend_from_slice(&command.context.stage_revision.get().to_be_bytes());
    bytes.extend_from_slice(command.context.context_digest.as_bytes());
    blob(&mut bytes, command.reason.as_str().as_bytes())?;
    match &command.action {
        MeasureAdministrativeAction::Correct(values) => {
            bytes.push(0);
            blob(&mut bytes, &values.canonical_bytes())?;
        }
        MeasureAdministrativeAction::MarkEnteredInError => bytes.push(1),
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id,
            subject,
        } => {
            bytes.push(2);
            bytes.extend_from_slice(replacement_id.as_uuid().as_bytes());
            bytes.extend_from_slice(subject.id.as_uuid().as_bytes());
            bytes.extend_from_slice(&subject.revision.get().to_be_bytes());
            bytes.extend_from_slice(subject.values_digest.as_bytes());
        }
    }
    Ok(bytes)
}

pub fn measure_administrative_review_bytes(
    review: &MeasureAdministrativeReview,
) -> Result<Vec<u8>, ApplicationError> {
    let replaces = matches!(
        review.command.action,
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace { .. }
    );
    if replaces != review.replacement.is_some() {
        return Err(invalid("replacement review shape differs from action"));
    }
    let mut bytes = b"MAPR1".to_vec();
    blob(
        &mut bytes,
        &measure_administrative_submission_bytes(&review.actor, review.case_id, &review.command)?,
    )?;
    bytes.extend_from_slice(review.submission_digest.as_bytes());
    blob(&mut bytes, &review.context.canonical_bytes())?;
    support(&mut bytes, &review.support)?;
    result(&mut bytes, &review.result)?;
    if let Some(replacement) = &review.replacement {
        result(&mut bytes, replacement)?;
    }
    Ok(bytes)
}

pub fn measure_administrative_record_bytes(
    record: &MeasureAdministrativeRecordCapture,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"MARCR1".to_vec();
    bytes.extend_from_slice(record.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(record.operation_id.as_uuid().as_bytes());
    result(&mut bytes, &record.result)?;
    actor(&mut bytes, &record.actor)?;
    blob(&mut bytes, &record.context.canonical_bytes())?;
    support(&mut bytes, &record.support)?;
    bytes.extend_from_slice(record.review_digest.as_bytes());
    timestamp(&mut bytes, record.recorded_at);
    Ok(bytes)
}

pub fn measure_administrative_capture_bytes(
    capture: &MeasureAdministrativeCapture,
) -> Result<Vec<u8>, ApplicationError> {
    let replaces = matches!(
        capture.review.command.action,
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace { .. }
    );
    if capture.records.len() > 32 || replaces != capture.replacement_link.is_some() {
        return Err(invalid("administrative capture shape differs from action"));
    }
    let mut bytes = b"MAGR1".to_vec();
    blob(
        &mut bytes,
        &measure_administrative_review_bytes(&capture.review)?,
    )?;
    bytes.extend_from_slice(capture.review.review_digest.as_bytes());
    bytes.extend_from_slice(&(capture.records.len() as u32).to_be_bytes());
    for row in &capture.records {
        blob(&mut bytes, &measure_administrative_record_bytes(row)?)?;
        bytes.extend_from_slice(row.capture_digest.as_bytes());
    }
    if let Some(link) = &capture.replacement_link {
        reference(&mut bytes, link.entered_in_error);
        reference(&mut bytes, link.replacement);
    }
    timestamp(&mut bytes, capture.recorded_at);
    Ok(bytes)
}
