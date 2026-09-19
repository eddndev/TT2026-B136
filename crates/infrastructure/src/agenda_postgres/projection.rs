use super::{header::Header, inconsistent};
use application::{agenda::*, deadlines::*, hearings::*, ApplicationError};
use domain::{clock::OffsetDateTime, crypto::DocumentHasher};
use postgres::Transaction;
use time::UtcOffset;

pub(super) fn item(
    tx: &mut Transaction<'_>,
    header: &Header,
    case: AgendaCaseSummary,
    hasher: &dyn DocumentHasher,
    checked_at: OffsetDateTime,
) -> Result<Option<AgendaItem>, ApplicationError> {
    match header.key.kind() {
        AgendaItemKind::Hearing => hearing(tx, header, case, hasher).map(Some),
        AgendaItemKind::Deadline => deadline(tx, header, case, hasher, checked_at),
    }
}

fn hearing(
    tx: &mut Transaction<'_>,
    header: &Header,
    case: AgendaCaseSummary,
    hasher: &dyn DocumentHasher,
) -> Result<AgendaItem, ApplicationError> {
    let detail = crate::hearing_postgres::storage::detail(
        tx,
        header.case,
        HearingId::from_uuid(header.key.id()),
        Some(HearingRevision::new(header.revision).map_err(inconsistent)?),
        hasher,
    )?;
    let snapshot = detail.snapshot;
    if snapshot.case_id != header.case
        || case.case_id != header.case
        || snapshot.id.as_uuid() != header.key.id()
        || snapshot.revision.get() != header.revision
        || snapshot.status.as_str() != header.status
        || snapshot.values.scheduled_at().utc() != header.key.at()
    {
        return Err(inconsistent(
            "hearing candidate differs from verified capture",
        ));
    }
    Ok(AgendaItem::Hearing(HearingOverview {
        case_id: header.case,
        case_title: case.title,
        case_reference: case.reference,
        case_status: case.status,
        id: snapshot.id,
        revision: snapshot.revision,
        kind: snapshot.values.kind(),
        scheduled_at: snapshot.values.scheduled_at(),
        modality: snapshot.values.modality(),
        status: snapshot.status,
        participant_count: snapshot.values.participants().len() as u8,
    }))
}

fn deadline(
    tx: &mut Transaction<'_>,
    header: &Header,
    case: AgendaCaseSummary,
    hasher: &dyn DocumentHasher,
    checked_at: OffsetDateTime,
) -> Result<Option<AgendaItem>, ApplicationError> {
    let detail = crate::deadline_postgres::storage::detail(
        tx,
        header.case,
        DeadlineId::from_uuid(header.key.id()),
        Some(DeadlineRevision::new(header.revision).map_err(inconsistent)?),
        hasher,
    )?;
    if detail.case_id != header.case
        || case.case_id != header.case
        || detail.id.as_uuid() != header.key.id()
        || detail.revision.get() != header.revision
        || detail.status.as_str() != header.status
        || detail
            .calculation
            .result
            .due_at()
            .map(|at| at.to_offset(UtcOffset::UTC))
            != Some(header.key.at())
    {
        return Err(inconsistent(
            "deadline candidate differs from verified capture",
        ));
    }
    let current =
        crate::deadline_postgres::current_in_transaction(tx, &detail, hasher, checked_at)?;
    if current.operational().due_at().is_none() {
        return Ok(None);
    }
    let overview = DeadlineOverview::from(&current);
    if !overview.operational.matches_overview(&overview) {
        return Err(inconsistent(
            "deadline overview differs from verified capture",
        ));
    }
    let item = AgendaItem::Deadline {
        case,
        deadline: Box::new(overview),
    };
    if item.key()? != header.key {
        return Err(inconsistent(
            "operational date differs from verified candidate",
        ));
    }
    Ok(Some(item))
}
