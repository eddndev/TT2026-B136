use super::*;
use crate::{
    deadline_inputs::extract_checked_deadline_inputs,
    deadline_profiles::deadline_profile_receipt_matches, deadline_tracking::DeadlineReviewState,
    ApplicationError,
};
use domain::{crypto::DocumentHasher, identity::Permission, procedural_facts::FactText};

/// Verify captured data and the operation receipt without rerunning historical arithmetic.
/// The store additionally reconstructs and verifies every exact observation reference.
pub fn deadline_receipt_matches(
    hasher: &dyn DocumentHasher,
    detail: &DeadlineDetail,
) -> Result<(), ApplicationError> {
    let receipt = &detail.receipt;
    shape(
        receipt,
        detail.revision,
        detail.status,
        detail.reason.as_ref(),
        &detail.recorded_by,
    )?;
    if receipt.action == DeadlineAction::Register && detail.attention != DeadlineAttention::Pending
    {
        return Err(inconsistent(
            "registration cannot include an attention declaration",
        ));
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
    match (&receipt.version, &detail.tracking) {
        (DeadlineReceiptVersion::Legacy, None) => {}
        (DeadlineReceiptVersion::Tracked(metadata), Some(tracking)) => {
            let digest = tracked_state::validate(hasher, definition, calculation, tracking)?;
            if digest != metadata.observations_digest {
                return Err(inconsistent("observations differ from receipt commitment"));
            }
            if matches!(
                receipt.action,
                DeadlineAction::Register | DeadlineAction::Correct
            ) && tracking.review.state() != DeadlineReviewState::Accepted
            {
                return Err(inconsistent(
                    "human qualification requires explicit acceptance",
                ));
            }
        }
        _ => {
            return Err(inconsistent(
                "receipt version and tracking capture disagree",
            ))
        }
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
    if canonical::state_digest(hasher, content, false)? != receipt.review_digest
        || canonical::state_digest(hasher, content, true)? != receipt.capture_digest
    {
        return Err(inconsistent(
            "reviewed or captured state differs from its digest",
        ));
    }
    if hasher.hash_bytes(&deadline_record_submission_bytes(detail)?) != receipt.submission_digest {
        return Err(inconsistent(
            "receipt digest differs from captured operation",
        ));
    }
    Ok(())
}

fn shape(
    receipt: &DeadlineReceipt,
    revision: DeadlineRevision,
    status: DeadlineStatus,
    reason: Option<&FactText>,
    author: &DeadlineActorSnapshot,
) -> Result<(), ApplicationError> {
    if receipt.expected_revision.checked_add(1) != Some(revision.get()) {
        return Err(inconsistent("receipt revision differs"));
    }
    if let Some(email) = author.email() {
        if email.is_empty()
            || email.trim() != email
            || email.chars().count() > 320
            || email.chars().any(char::is_control)
        {
            return Err(inconsistent("captured actor email is not canonical"));
        }
    }
    let valid = match receipt.action {
        DeadlineAction::Register => {
            receipt.expected_revision == 0 && reason.is_none() && status == DeadlineStatus::Active
        }
        DeadlineAction::Correct | DeadlineAction::SetAttention | DeadlineAction::Reevaluate => {
            receipt.expected_revision > 0 && reason.is_some() && status == DeadlineStatus::Active
        }
        DeadlineAction::Retire => {
            receipt.expected_revision > 0 && reason.is_some() && status == DeadlineStatus::Retired
        }
    };
    if !valid {
        return Err(inconsistent("receipt action and registry state disagree"));
    }
    Ok(())
}

/// The store verifies each lightweight projection against the full captured state.
pub fn deadline_history_receipt_matches(
    hasher: &dyn DocumentHasher,
    entry: &DeadlineHistoryEntry,
) -> Result<(), ApplicationError> {
    let receipt = &entry.receipt;
    shape(
        receipt,
        entry.revision,
        entry.status,
        entry.reason.as_ref(),
        &entry.recorded_by,
    )?;
    if entry.state_digest != receipt.review_digest
        || hasher.hash_bytes(&tracked::submission_bytes(
            &entry.recorded_by,
            entry.case_id,
            entry.id,
            receipt,
            entry.reason.as_ref(),
        )?) != receipt.submission_digest
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
