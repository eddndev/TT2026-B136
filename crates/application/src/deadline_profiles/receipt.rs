use super::*;
use crate::ApplicationError;
use domain::crypto::DocumentHasher;
pub(super) fn inconsistent(message: &str) -> ApplicationError {
    DeadlineProfileError::StoredInconsistent(message.into()).into()
}
/// Validate the lightweight receipt. The definition digest binds the immutable scope
/// indirectly; storage must validate its scope projection against DPRF and the root.
/// Storage also resolves prior revisions to enforce terminal status and preservation.
pub fn deadline_profile_history_receipt_matches(
    hasher: &dyn DocumentHasher,
    entry: &DeadlineProfileHistoryEntry,
) -> Result<(), ApplicationError> {
    let receipt = &entry.receipt;
    let shape = match receipt.action {
        DeadlineProfileAction::Publish => {
            receipt.expected_revision == 0
                && entry.reason.is_none()
                && entry.status == DeadlineProfileStatus::Published
        }
        DeadlineProfileAction::Replace => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == DeadlineProfileStatus::Published
        }
        DeadlineProfileAction::Retire => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == DeadlineProfileStatus::Retired
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
pub fn deadline_profile_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &DeadlineProfileDetail,
) -> Result<(), ApplicationError> {
    deadline_profile_history_receipt_matches(hasher, &DeadlineProfileHistoryEntry::from(detail))?;
    if deadline_profile_definition_digest(hasher, &detail.definition) != detail.definition_digest {
        return Err(inconsistent("profile definition differs from its digest"));
    }
    Ok(())
}
impl From<&DeadlineProfileDetail> for DeadlineProfileHistoryEntry {
    fn from(v: &DeadlineProfileDetail) -> Self {
        Self {
            id: v.id,
            revision: v.revision,
            status: v.status,
            definition_digest: v.definition_digest,
            algorithm: v.algorithm,
            scope: v.definition.scope().clone(),
            reason: v.reason.clone(),
            receipt: v.receipt.clone(),
            recorded_at: v.recorded_at,
            recorded_by: v.recorded_by.clone(),
        }
    }
}
impl From<&DeadlineProfileDetail> for DeadlineProfileOverview {
    fn from(v: &DeadlineProfileDetail) -> Self {
        Self {
            id: v.id,
            revision: v.revision,
            status: v.status,
            algorithm: v.algorithm,
            definition_digest: v.definition_digest,
            title: v.definition.title().clone(),
            scope: v.definition.scope().clone(),
        }
    }
}
