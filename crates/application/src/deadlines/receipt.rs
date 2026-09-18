use super::*;
use crate::{
    deadline_inputs::extract_checked_deadline_inputs,
    deadline_profiles::deadline_profile_receipt_matches, ApplicationError,
};
use domain::{crypto::DocumentHasher, identity::Permission};

/// Verify captured data and the operation receipt without rerunning historical arithmetic.
pub fn deadline_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    let receipt = &detail.receipt;
    if receipt.expected_revision.checked_add(1) != Some(detail.revision.get())
        || detail.recorded_by.email.trim().is_empty()
    {
        return Err(inconsistent("receipt revision or actor differs"));
    }
    let shape = match receipt.action {
        DeadlineAction::Register => {
            receipt.expected_revision == 0
                && detail.reason.is_none()
                && detail.attention == DeadlineAttention::Pending
                && detail.status == DeadlineStatus::Active
        }
        DeadlineAction::Correct | DeadlineAction::SetAttention => {
            receipt.expected_revision > 0
                && detail.reason.is_some()
                && detail.status == DeadlineStatus::Active
        }
        DeadlineAction::Retire => {
            receipt.expected_revision > 0
                && detail.reason.is_some()
                && detail.status == DeadlineStatus::Retired
        }
    };
    if !shape {
        return Err(inconsistent("receipt action and registry state disagree"));
    }
    let definition = &detail.definition;
    let calculation = &detail.calculation;
    if definition.input.selection.case_id != detail.case_id
        || calculation.material.case_id != detail.case_id
        || calculation.profile.id != definition.profile.id
        || calculation.profile.revision != definition.profile.revision
        || detail.responsible.id != definition.responsible
        || detail.responsible.email.trim().is_empty()
        || !detail.responsible.role.allows(Permission::ReadDeadline)
    {
        return Err(inconsistent(
            "captured identity, profile or responsible differs",
        ));
    }
    deadline_profile_receipt_matches(hasher, &calculation.profile)?;
    let extraction = extract_checked_deadline_inputs(
        hasher,
        calculation.profile.definition.trigger(),
        &definition.input.selection,
        definition.input.calendar,
        &calculation.material,
    )?;
    if calculation.result.requirement() != calculation.profile.definition.trigger()
        || calculation.result.trigger_outcome() != extraction.extraction().outcome()
    {
        return Err(inconsistent("captured trigger differs from exact input"));
    }
    let content = canonical::Content::from(detail);
    let state = canonical::state_digest(hasher, content, false)?;
    if state != receipt.review_digest
        || canonical::state_digest(hasher, content, true)? != receipt.capture_digest
    {
        return Err(inconsistent(
            "reviewed or captured state differs from its digest",
        ));
    }
    if canonical::submission_digest(
        hasher,
        detail.recorded_by.id,
        detail.case_id,
        detail.id,
        receipt,
        state,
        detail.reason.as_ref(),
    ) != receipt.submission_digest
    {
        return Err(inconsistent(
            "receipt digest differs from captured operation",
        ));
    }
    Ok(())
}

/// The store verifies each lightweight projection against the full captured state.
pub fn deadline_history_receipt_matches(
    hasher: &dyn DocumentHasher,
    entry: &DeadlineHistoryEntry,
) -> Result<(), ApplicationError> {
    let receipt = &entry.receipt;
    if entry.state_digest != receipt.review_digest
        || receipt.expected_revision.checked_add(1) != Some(entry.revision.get())
        || entry.recorded_by.email.trim().is_empty()
    {
        return Err(inconsistent("history receipt or actor differs"));
    }
    let shape = match receipt.action {
        DeadlineAction::Register => {
            receipt.expected_revision == 0
                && entry.reason.is_none()
                && entry.status == DeadlineStatus::Active
        }
        DeadlineAction::Correct | DeadlineAction::SetAttention => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == DeadlineStatus::Active
        }
        DeadlineAction::Retire => {
            receipt.expected_revision > 0
                && entry.reason.is_some()
                && entry.status == DeadlineStatus::Retired
        }
    };
    if !shape
        || canonical::submission_digest(
            hasher,
            entry.recorded_by.id,
            entry.case_id,
            entry.id,
            receipt,
            entry.state_digest,
            entry.reason.as_ref(),
        ) != receipt.submission_digest
    {
        return Err(inconsistent("history receipt differs from its operation"));
    }
    Ok(())
}
impl DeadlineHistoryEntry {
    pub fn from_detail(
        hasher: &dyn DocumentHasher,
        detail: &DeadlineDetail,
    ) -> Result<Self, ApplicationError> {
        deadline_receipt_matches(hasher, detail)?;
        Ok(Self {
            id: detail.id,
            case_id: detail.case_id,
            revision: detail.revision,
            status: detail.status,
            reason: detail.reason.clone(),
            receipt: detail.receipt.clone(),
            state_digest: detail.receipt.review_digest,
            recorded_at: detail.recorded_at,
            recorded_by: detail.recorded_by.clone(),
        })
    }
}
