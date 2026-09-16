use super::{receipt::inconsistent, *};
use crate::ApplicationError;
use domain::crypto::DocumentHasher;
pub(super) fn validate_preparation(
    hasher: &dyn DocumentHasher,
    command: &JudicialCalendarCommand,
    preparation: &JudicialCalendarPreparation,
) -> Result<JudicialCalendarValues, ApplicationError> {
    if preparation.calendar_id != command.calendar_id {
        return Err(inconsistent("prepared root differs from requested root"));
    }
    match (&preparation.base, &preparation.initial_scope) {
        (None, None) => {}
        (Some(base), Some(scope)) => {
            judicial_calendar_receipt_matches(hasher, base)?;
            if base.id != command.calendar_id || base.values.scope() != scope {
                return Err(inconsistent("prepared base or R1 scope differs"));
            }
        }
        _ => return Err(inconsistent("base and R1 scope must occur together")),
    }
    if let JudicialCalendarChange::Publish { values } = &command.change {
        if preparation.base.is_some() {
            return Err(JudicialCalendarError::RevisionConflict.into());
        }
        return Ok(values.clone());
    }
    let base = preparation
        .base
        .as_ref()
        .ok_or(JudicialCalendarError::NotFound)?;
    if base.revision.get() != command.expected_revision() {
        return Err(JudicialCalendarError::RevisionConflict.into());
    }
    if base.status == JudicialCalendarStatus::Retired {
        return Err(JudicialCalendarError::Retired.into());
    }
    match &command.change {
        JudicialCalendarChange::Replace { values, .. } => {
            if Some(values.scope()) != preparation.initial_scope.as_ref() {
                return Err(JudicialCalendarError::ScopeChangeForbidden.into());
            }
            Ok(values.clone())
        }
        JudicialCalendarChange::Retire { .. } => Ok(base.values.clone()),
        JudicialCalendarChange::Publish { .. } => unreachable!(),
    }
}
pub(super) fn validate_page(
    page: &JudicialCalendarPage,
    query: &JudicialCalendarQuery,
) -> Result<(), ApplicationError> {
    if page.calendars.len() > query.limit() as usize
        || page.has_more && page.calendars.len() != query.limit() as usize
        || page.next_after_id
            != if page.has_more {
                page.calendars.last().map(|e| e.id)
            } else {
                None
            }
    {
        return Err(inconsistent("calendar page size or cursor differs"));
    }
    let mut after = query.after_id();
    for e in &page.calendars {
        if after.is_some_and(|id| e.id.as_uuid() <= id.as_uuid())
            || query.status().status().is_some_and(|s| e.status != s)
            || query
                .jurisdiction()
                .is_some_and(|j| e.scope.jurisdiction() != j)
            || query
                .entity_code()
                .is_some_and(|c| !e.scope.entity_codes().iter().any(|v| v == c))
        {
            return Err(inconsistent(
                "calendar page order or requested filter differs",
            ));
        }
        after = Some(e.id);
    }
    Ok(())
}
pub(super) fn validate_history(
    hasher: &dyn DocumentHasher,
    id: JudicialCalendarId,
    page: &JudicialCalendarHistoryPage,
    query: JudicialCalendarHistoryQuery,
) -> Result<(), ApplicationError> {
    if page.revisions.is_empty() && query.before_revision().map(|v| v.get()) != Some(1) {
        return Err(inconsistent("calendar history omits its first revision"));
    }
    if page.revisions.len() > query.limit() as usize
        || page.has_more && page.revisions.len() != query.limit() as usize
        || page.next_before_revision
            != if page.has_more {
                page.revisions.last().map(|e| e.revision)
            } else {
                None
            }
        || page.has_more && page.revisions.last().is_some_and(|e| e.revision.get() == 1)
    {
        return Err(inconsistent("calendar history size or cursor differs"));
    }
    let mut operations = std::collections::HashSet::new();
    for e in &page.revisions {
        if !operations.insert(e.receipt.operation_id) {
            return Err(inconsistent("calendar history repeats an operation"));
        }
        judicial_calendar_history_receipt_matches(hasher, e)?;
        if e.id != id
            || query
                .before_revision()
                .is_some_and(|before| e.revision >= before)
        {
            return Err(inconsistent("calendar history root or bound differs"));
        }
    }
    for pair in page.revisions.windows(2) {
        if Some(pair[0].revision.get()) != pair[1].revision.get().checked_add(1)
            || pair[1].status == JudicialCalendarStatus::Retired
            || pair[0].receipt.operation_id == pair[1].receipt.operation_id
            || pair[0].status == JudicialCalendarStatus::Retired
                && pair[0].values_digest != pair[1].values_digest
        {
            return Err(inconsistent(
                "calendar history sequence or retirement differs",
            ));
        }
    }
    if !page.has_more && page.revisions.last().is_some_and(|e| e.revision.get() != 1) {
        return Err(inconsistent(
            "calendar history ends before its first revision",
        ));
    }
    Ok(())
}
