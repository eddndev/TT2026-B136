use super::*;
use crate::{
    cases::{CaseAdministrativeStatus, CurrentCaseAdministration},
    ApplicationError,
};
use domain::{cases::CaseId, crypto::DocumentHasher, identity::Permission};
use std::collections::HashSet;

pub(super) fn validate_commit(
    hasher: &dyn DocumentHasher,
    reviewed: &DeadlineDraft,
    detail: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    deadline_receipt_matches(hasher, detail)?;
    let command = &reviewed.command;
    let receipt = &detail.receipt;
    if detail.case_id != reviewed.case_id
        || detail.id != command.deadline_id
        || detail.revision != reviewed.result_revision
        || detail.recorded_by.id != reviewed.actor
        || receipt.operation_id != command.operation_id
        || receipt.action != command.action()
        || receipt.expected_revision != command.expected_revision()
        || receipt.review_digest != reviewed.review_digest
        || receipt.submission_digest != reviewed.submission_digest
        || detail.definition != reviewed.definition
        || detail.responsible != reviewed.responsible
        || detail.attention != reviewed.attention
        || detail.status != reviewed.status
        || detail.reason.as_ref() != command.reason()
        || detail.calculation.profile != reviewed.calculation.profile
        || detail.calculation.result != reviewed.calculation.result
    {
        return Err(inconsistent(
            "committed deadline differs from the reviewed operation or calculation",
        ));
    }
    let old = &reviewed.calculation.material;
    let new = &detail.calculation.material;
    if old.case_id != new.case_id
        || old.source != new.source
        || old.source_head != new.source_head
        || old.calendar != new.calendar
        || old.calendar_head != new.calendar_head
    {
        return Err(inconsistent(
            "committed deadline changed exact sources or observed heads",
        ));
    }
    match command.action() {
        DeadlineAction::Register | DeadlineAction::Correct => {
            validate_administration(&old.administration, &new.administration)?
        }
        DeadlineAction::SetAttention | DeadlineAction::Retire => {
            if old.administration != new.administration
                || receipt.capture_digest != reviewed.capture_digest
            {
                return Err(inconsistent(
                    "attention or retirement changed the historical calculation capture",
                ));
            }
        }
    }
    Ok(())
}
fn validate_administration(
    old: &CurrentCaseAdministration,
    new: &CurrentCaseAdministration,
) -> Result<(), ApplicationError> {
    if new.values().status() != CaseAdministrativeStatus::Active {
        return Err(inconsistent(
            "committed deadline captured a closed case administration",
        ));
    }
    let valid = match (old, new) {
        (
            CurrentCaseAdministration::Unrevised(before),
            CurrentCaseAdministration::Unrevised(after),
        ) => before == after,
        (CurrentCaseAdministration::Unrevised(_), CurrentCaseAdministration::Recorded(_)) => true,
        (CurrentCaseAdministration::Recorded(_), CurrentCaseAdministration::Unrevised(_)) => false,
        (
            CurrentCaseAdministration::Recorded(before),
            CurrentCaseAdministration::Recorded(after),
        ) => {
            after.revision > before.revision
                || (after == before && after.changed_at.offset() == before.changed_at.offset())
        }
    };
    if !valid {
        return Err(inconsistent(
            "committed administration moved backwards or rewrote an immutable revision",
        ));
    }
    Ok(())
}

pub(super) fn validate_page(
    case_id: CaseId,
    page: &DeadlinePage,
    query: &DeadlineQuery,
) -> Result<(), ApplicationError> {
    if page.deadlines.len() > query.limit() as usize
        || (page.has_more
            && (page.deadlines.len() != query.limit() as usize
                || page.next_after_id != page.deadlines.last().map(|row| row.id)))
        || (!page.has_more && page.next_after_id.is_some())
    {
        return Err(inconsistent(
            "deadline page size or cursor differs from its pagination state",
        ));
    }
    let mut previous = query.after_id();
    for row in &page.deadlines {
        if row.case_id != case_id
            || query
                .status()
                .status()
                .is_some_and(|status| row.status != status)
            || previous.is_some_and(|id| row.id.as_uuid() <= id.as_uuid())
            || !row.responsible.role.allows(Permission::ReadDeadline)
            || row.responsible.email.trim().is_empty()
            || row.blocked != row.due_at.is_none()
        {
            return Err(inconsistent(
                "deadline summary scope, order, status, responsible or outcome is inconsistent",
            ));
        }
        previous = Some(row.id);
    }
    Ok(())
}
pub(super) fn validate_history(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    id: DeadlineId,
    page: &DeadlineHistoryPage,
    query: DeadlineHistoryQuery,
) -> Result<(), ApplicationError> {
    if page.revisions.len() > query.limit() as usize
        || (page.has_more
            && (page.revisions.len() != query.limit() as usize
                || page.next_before_revision != page.revisions.last().map(|row| row.revision)
                || page
                    .revisions
                    .last()
                    .is_some_and(|row| row.revision.get() == 1)))
        || (!page.has_more && page.next_before_revision.is_some())
    {
        return Err(inconsistent(
            "deadline history size or cursor differs from its pagination state",
        ));
    }
    if page.revisions.is_empty()
        && query.before_revision().map(|revision| revision.get()) != Some(1)
    {
        return Err(inconsistent(
            "empty deadline history omits the existing initial revision",
        ));
    }
    let mut operations = HashSet::new();
    for row in &page.revisions {
        deadline_history_receipt_matches(hasher, row)?;
        if row.case_id != case_id
            || row.id != id
            || !operations.insert(row.receipt.operation_id)
            || query
                .before_revision()
                .is_some_and(|before| row.revision >= before)
        {
            return Err(inconsistent(
                "deadline history scope, operation or exclusive revision bound differs",
            ));
        }
    }
    for pair in page.revisions.windows(2) {
        if pair[1].revision.get().checked_add(1) != Some(pair[0].revision.get())
            || pair[1].status == DeadlineStatus::Retired
        {
            return Err(inconsistent(
                "deadline history is not consecutive or continues after retirement",
            ));
        }
    }
    if !page.has_more
        && page
            .revisions
            .last()
            .is_some_and(|row| row.revision.get() != 1)
    {
        return Err(inconsistent(
            "deadline history ends before its initial revision",
        ));
    }
    Ok(())
}
