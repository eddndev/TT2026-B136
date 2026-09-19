use super::{DocumentIntegrityIncident, DocumentIntegrityPage, DocumentIntegrityQuery};
use crate::ApplicationError;
use domain::clock::OffsetDateTime;
use std::collections::HashSet;

pub(super) fn inconsistent(message: &str) -> ApplicationError {
    ApplicationError::StoredDocumentInconsistent(message.into())
}

pub(super) fn window(start: OffsetDateTime, end: OffsetDateTime) -> Result<(), ApplicationError> {
    if !utc(start) || !utc(end) || end < start {
        return Err(inconsistent("incident query clock window is invalid"));
    }
    Ok(())
}

pub(super) fn incident(
    record: &DocumentIntegrityIncident,
    upper: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if record.id.as_uuid().is_nil()
        || record.observation_id.as_uuid().is_nil()
        || record.case_id.as_uuid().is_nil()
        || record.reference.id.as_uuid().is_nil()
        || record.requester.as_uuid().is_nil()
        || !utc(record.detected_at)
        || !utc(record.recorded_at)
        || record.detected_at > record.recorded_at
        || record.recorded_at > upper
    {
        return Err(inconsistent(
            "incident projection contains invalid identities or times",
        ));
    }
    Ok(())
}

pub(super) fn page(
    page: &DocumentIntegrityPage,
    query: DocumentIntegrityQuery,
    upper: OffsetDateTime,
) -> Result<(), ApplicationError> {
    if page.incidents.len() > query.limit() as usize
        || (page.has_more && page.incidents.len() != query.limit() as usize)
        || page.next_after_id
            != if page.has_more {
                page.incidents.last().map(|row| row.id)
            } else {
                None
            }
    {
        return Err(inconsistent("incident page size or continuation differs"));
    }
    let mut previous = query.after_id();
    let mut observations = HashSet::new();
    for record in &page.incidents {
        incident(record, upper)?;
        if previous.is_some_and(|id| id >= record.id) || !observations.insert(record.observation_id)
        {
            return Err(inconsistent(
                "incident page order or observation uniqueness differs",
            ));
        }
        previous = Some(record.id);
    }
    Ok(())
}

fn utc(at: OffsetDateTime) -> bool {
    at.offset() == time::UtcOffset::UTC && (1..=9999).contains(&at.year())
}
