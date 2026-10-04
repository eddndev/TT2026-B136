use super::stored;
use application::{alerts::*, resource_hearings::ResourceHearingDetail, ApplicationError};
use domain::{crypto::DocumentHasher, resource_hearings::ResourceHearingRevision};
use postgres::Transaction;

pub(super) fn detail(
    tx: &mut Transaction<'_>,
    subject: AlertSubject,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceHearingDetail, ApplicationError> {
    let AlertSubject::ResourceHearing {
        case_id,
        resource_id,
        id,
    } = subject
    else {
        return Err(stored("expected resource hearing alert subject"));
    };
    let detail = crate::resource_hearing_postgres::storage::detail(
        tx,
        case_id,
        id,
        Some(ResourceHearingRevision::initial()),
        hasher,
    )
    .map_err(stored)?;
    let creation =
        crate::resource_hearing_postgres::creation(tx, detail, hasher).map_err(stored)?;
    let detail = creation.hearing;
    if detail.review.case_id != case_id
        || detail.review.command.resource.id != resource_id
        || detail.review.command.hearing_id != id
        || detail.revision != ResourceHearingRevision::initial()
    {
        return Err(stored("resource hearing alert parent or identity differs"));
    }
    Ok(detail)
}

pub(super) fn title(detail: &ResourceHearingDetail) -> String {
    format!(
        "Audiencia de recurso {}",
        detail.review.command.values.kind().as_str()
    )
}

pub(super) fn origin(
    tx: &mut Transaction<'_>,
    record: &AlertRecord,
    hasher: &dyn DocumentHasher,
) -> Result<ResourceHearingDetail, ApplicationError> {
    let detail = detail(tx, record.subject, hasher)?;
    if record.origin.revision != 1
        || record.origin.evidence_digest != detail.capture_digest
        || !matches!(record.kind, AlertKind::Upcoming { activity_at, .. }
            if activity_at == detail.review.command.values.scheduled_at().utc())
    {
        return Err(stored("resource hearing alert origin or activity differs"));
    }
    Ok(detail)
}
