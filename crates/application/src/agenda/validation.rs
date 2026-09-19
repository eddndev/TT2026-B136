use super::{
    query::valid_instant, AgendaCursor, AgendaItem, AgendaItemKind, AgendaPage, AgendaQuery,
};
use crate::{
    deadline_currentness::DeadlineFreshness,
    deadline_tracking::DeadlineReviewState,
    deadlines::{DeadlineOverview, DeadlineReceiptKind, DeadlineStatus},
    ApplicationError,
};
use domain::{
    cases::CaseMetadata, clock::OffsetDateTime, hearings::MAX_HEARING_PARTICIPANTS,
    identity::Permission,
};
use std::collections::HashSet;
use time::UtcOffset;

pub(super) fn item_key(item: &AgendaItem) -> Result<AgendaCursor, ApplicationError> {
    let (at, kind, id) = match item {
        AgendaItem::Hearing(row) => {
            metadata(&row.case_title, &row.case_reference)?;
            if usize::from(row.participant_count) > MAX_HEARING_PARTICIPANTS {
                return Err(inconsistent());
            }
            (
                row.scheduled_at.utc(),
                AgendaItemKind::Hearing,
                row.id.as_uuid(),
            )
        }
        AgendaItem::Deadline { case, deadline } => {
            metadata(&case.title, &case.reference)?;
            if case.case_id != deadline.case_id {
                return Err(inconsistent());
            }
            (
                deadline_date(deadline)?,
                AgendaItemKind::Deadline,
                deadline.id.as_uuid(),
            )
        }
    };
    AgendaCursor::new(at, kind, id).map_err(|_| inconsistent())
}

pub(super) fn validate_page(
    page: &AgendaPage,
    query: &AgendaQuery,
) -> Result<(), ApplicationError> {
    if !valid_instant(page.checked_at)
        || page.items.len() > query.limit() as usize
        || page.complete != page.next_after.is_none()
    {
        return Err(inconsistent());
    }
    let mut previous = query.after();
    let mut identities = HashSet::new();
    for item in &page.items {
        let key = item_key(item)?;
        let family = match key.kind() {
            AgendaItemKind::Hearing => 0_u8,
            AgendaItemKind::Deadline => 1_u8,
        };
        if !query.accepts(key)
            || previous.is_some_and(|value| key <= value)
            || !identities.insert((family, key.id()))
        {
            return Err(inconsistent());
        }
        match item {
            AgendaItem::Hearing(row) => {
                if query
                    .hearing_status()
                    .status()
                    .is_some_and(|status| row.status != status)
                {
                    return Err(inconsistent());
                }
            }
            AgendaItem::Deadline { deadline, .. } => {
                if deadline.operational.checked_at() != Some(page.checked_at)
                    || deadline.operational.checked_at().map(|at| at.offset())
                        != Some(UtcOffset::UTC)
                {
                    return Err(inconsistent());
                }
            }
        }
        previous = Some(key);
    }
    if let Some(cursor) = page.next_after {
        if !query.accepts(cursor)
            || query.after().is_some_and(|before| cursor <= before)
            || previous.is_some_and(|last| cursor < last)
        {
            return Err(inconsistent());
        }
    }
    Ok(())
}

fn deadline_date(row: &DeadlineOverview) -> Result<OffsetDateTime, ApplicationError> {
    if !row.operational.matches_overview(row)
        || row.status != DeadlineStatus::Active
        || row.receipt_kind != DeadlineReceiptKind::Tracked
        || row.review_state != DeadlineReviewState::Accepted
        || row.operational.freshness() != DeadlineFreshness::Current
        || row.calculation_blocked
        || !row.responsible.role.allows(Permission::ReadDeadline)
        || row.responsible.email.is_empty()
        || row.responsible.email.trim() != row.responsible.email
        || row.responsible.email.chars().any(char::is_control)
        || !row.operational.checked_at().is_some_and(valid_instant)
    {
        return Err(inconsistent());
    }
    let due = row.operational.due_at().ok_or_else(inconsistent)?;
    if row.calculation_due_at != Some(due)
        || row.calculation_due_at.map(|at| at.offset()) != Some(due.offset())
    {
        return Err(inconsistent());
    }
    due.checked_to_offset(UtcOffset::UTC)
        .ok_or_else(inconsistent)
}

fn metadata(title: &str, reference: &str) -> Result<(), ApplicationError> {
    let value = CaseMetadata::new(title, reference).map_err(|_| inconsistent())?;
    if value.title() != title || value.reference() != reference {
        return Err(inconsistent());
    }
    Ok(())
}

fn inconsistent() -> ApplicationError {
    ApplicationError::Port(
        "agenda projection differs from its query, observation or verified capture".into(),
    )
}
