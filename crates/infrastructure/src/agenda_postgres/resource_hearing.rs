use super::{header::Header, inconsistent};
use application::{agenda::*, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher, resource_hearings::*};
use postgres::Transaction;

pub(super) fn item(
    tx: &mut Transaction<'_>,
    header: &Header,
    case: AgendaCaseSummary,
    hasher: &dyn DocumentHasher,
    checked_at: OffsetDateTime,
) -> Result<AgendaItem, ApplicationError> {
    let detail = crate::resource_hearing_postgres::storage::detail(
        tx,
        header.case,
        ResourceHearingId::from_uuid(header.key.id()),
        Some(ResourceHearingRevision::new(header.revision).map_err(inconsistent)?),
        hasher,
    )
    .map_err(inconsistent)?;
    let creation =
        crate::resource_hearing_postgres::creation(tx, detail, hasher).map_err(inconsistent)?;
    let detail = &creation.hearing;
    let command = &detail.review.command;
    if case.case_id != header.case
        || detail.review.case_id != header.case
        || command.hearing_id.as_uuid() != header.key.id()
        || Some(command.resource.id) != header.resource
        || detail.revision != ResourceHearingRevision::initial()
        || detail.revision.get() != header.revision
        || header.status.is_some()
        || command.values.scheduled_at().utc() != header.key.at()
        || detail.recorded_at > checked_at
    {
        return Err(inconsistent(
            "resource hearing candidate differs from verified capture",
        ));
    }
    Ok(AgendaItem::ResourceHearing {
        case,
        hearing: Box::new(ResourceHearingAgendaOverview {
            case_id: header.case,
            resource_id: command.resource.id,
            id: command.hearing_id,
            revision: detail.revision,
            kind: command.values.kind(),
            scheduled_at: command.values.scheduled_at(),
            modality: command.values.modality(),
            participant_count: command.values.participants().len() as u8,
            association_id: creation.origin.association_id,
            capture_digest: creation.origin.capture_digest,
        }),
    })
}
