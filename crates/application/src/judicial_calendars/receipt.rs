use super::*;
use crate::ApplicationError;
use domain::crypto::DocumentHasher;
pub(super) fn inconsistent(message: &str) -> ApplicationError {
    JudicialCalendarError::StoredInconsistent(message.into()).into()
}
/// Validate a self-contained envelope; storage must additionally resolve R1 and prior revisions.
pub fn judicial_calendar_history_receipt_matches(
    hasher: &dyn DocumentHasher,
    entry: &JudicialCalendarHistoryEntry,
) -> Result<(), ApplicationError> {
    let receipt = &entry.receipt;
    let shape = match receipt.action {
        JudicialCalendarAction::Publish => {
            receipt.expected_revision == 0
                && entry.reason.is_none()
                && entry.status == JudicialCalendarStatus::Published
        }
        JudicialCalendarAction::Replace => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == JudicialCalendarStatus::Published
        }
        JudicialCalendarAction::Retire => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == JudicialCalendarStatus::Retired
        }
    };
    if !shape
        || receipt.expected_revision.checked_add(1) != Some(entry.revision.get())
        || entry.recorded_by.email.trim().is_empty()
    {
        return Err(inconsistent(
            "receipt action, revision, status or actor projection differs",
        ));
    }
    if hasher.hash_bytes(&super::canonical::history_submission_bytes(entry))
        != receipt.submission_digest
    {
        return Err(inconsistent(
            "receipt digest differs from reconstructed operation",
        ));
    }
    Ok(())
}
pub fn judicial_calendar_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &JudicialCalendarDetail,
) -> Result<(), ApplicationError> {
    judicial_calendar_history_receipt_matches(hasher, &JudicialCalendarHistoryEntry::from(detail))?;
    if judicial_calendar_values_digest(hasher, &detail.values) != detail.values_digest {
        return Err(inconsistent("calendar values differ from their digest"));
    }
    Ok(())
}
impl From<&JudicialCalendarDetail> for JudicialCalendarHistoryEntry {
    fn from(v: &JudicialCalendarDetail) -> Self {
        Self {
            id: v.id,
            revision: v.revision,
            status: v.status,
            values_digest: v.values_digest,
            reason: v.reason.clone(),
            receipt: v.receipt.clone(),
            recorded_at: v.recorded_at,
            recorded_by: v.recorded_by.clone(),
        }
    }
}
impl From<&JudicialCalendarDetail> for JudicialCalendarOverview {
    fn from(v: &JudicialCalendarDetail) -> Self {
        Self {
            id: v.id,
            revision: v.revision,
            status: v.status,
            values_digest: v.values_digest,
            scope: v.values.scope().clone(),
            coverage: v.values.coverage(),
            has_unresolved: v.values.has_unresolved(),
        }
    }
}
