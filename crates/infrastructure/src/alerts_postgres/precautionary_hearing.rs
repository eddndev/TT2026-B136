use super::stored;
use application::{
    alerts::*, precautionary_hearings::PrecautionaryHearingCapture, ApplicationError,
};
use domain::{
    cases::CaseMetadata, crypto::DocumentHasher, hearings::HearingStatus,
    precautionary_hearings::PrecautionaryHearingRevision,
};
use postgres::Transaction;

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    subject: AlertSubject,
    revision: Option<u32>,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingCapture, ApplicationError> {
    let AlertSubject::PrecautionaryHearing { case_id, id } = subject else {
        return Err(stored("expected precautionary hearing alert subject"));
    };
    let selected = revision
        .map(PrecautionaryHearingRevision::new)
        .transpose()
        .map_err(stored)?;
    let capture = crate::precautionary_hearing_postgres::record_storage::detail(
        tx, case_id, id, selected, hasher,
    )
    .map_err(stored)?
    .capture;
    if capture.review.case_id != case_id
        || capture.review.command.hearing_id != id
        || selected.is_some_and(|revision| capture.review.result_revision != revision)
    {
        return Err(stored("precautionary alert identity or revision differs"));
    }
    Ok(capture)
}
pub(super) fn title(capture: &PrecautionaryHearingCapture) -> String {
    format!(
        "Audiencia cautelar {}",
        capture.review.resolved_values.purpose().as_str()
    )
}
pub(super) fn metadata(capture: &PrecautionaryHearingCapture) -> &CaseMetadata {
    capture
        .review
        .observed_context
        .material()
        .administration
        .values
        .metadata()
}
pub(super) fn origin(
    tx: &mut Transaction<'_>,
    record: &AlertRecord,
    hasher: &dyn DocumentHasher,
) -> Result<PrecautionaryHearingCapture, ApplicationError> {
    let capture = detail(tx, record.subject, Some(record.origin.revision), hasher)?;
    if record.origin.evidence_digest != capture.capture_digest
        || capture.review.status != HearingStatus::Scheduled
        || !matches!(record.kind, AlertKind::Upcoming { activity_at, .. }
            if activity_at == capture.review.resolved_values.scheduled_at().utc())
    {
        return Err(stored("precautionary alert origin or activity differs"));
    }
    Ok(capture)
}
