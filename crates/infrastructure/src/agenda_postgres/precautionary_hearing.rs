use super::{header::Header, inconsistent};
use application::{agenda::*, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher, precautionary_hearings::*};
use postgres::Transaction;

pub(super) fn item(
    tx: &mut Transaction<'_>,
    header: &Header,
    case: AgendaCaseSummary,
    hasher: &dyn DocumentHasher,
    checked_at: OffsetDateTime,
) -> Result<AgendaItem, ApplicationError> {
    let detail = crate::precautionary_hearing_postgres::record_storage::detail(
        tx,
        header.case,
        PrecautionaryHearingId::from_uuid(header.key.id()),
        Some(PrecautionaryHearingRevision::new(header.revision).map_err(inconsistent)?),
        hasher,
    )
    .map_err(inconsistent)?;
    let capture = detail.capture;
    let review = &capture.review;
    let values = &review.resolved_values;
    if case.case_id != header.case
        || review.case_id != header.case
        || review.command.hearing_id.as_uuid() != header.key.id()
        || review.result_revision.get() != header.revision
        || Some(review.status.as_str()) != header.status.as_deref()
        || values.scheduled_at().utc() != header.key.at()
        || capture.recorded_at > checked_at
    {
        return Err(inconsistent(
            "precautionary candidate differs from verified capture",
        ));
    }
    Ok(AgendaItem::PrecautionaryHearing {
        case,
        hearing: Box::new(PrecautionaryHearingAgendaOverview {
            case_id: header.case,
            id: review.command.hearing_id,
            revision: review.result_revision,
            purpose: values.purpose(),
            scheduled_at: values.scheduled_at(),
            modality: values.modality(),
            status: review.status,
            participant_count: values.participants().len() as u8,
            capture_digest: capture.capture_digest,
        }),
    })
}
