use super::*;
use crate::{
    deadline_reevaluation::SourceEventReference,
    deadlines::{deadline_record_submission_bytes, DeadlineDetail},
    hearing_results::HearingResultDetail,
};

/// Validated final evidence, not proof that a storage transaction committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineCreation {
    pub(super) draft: HearingDerivedDeadlineDraft,
    pub(super) result: HearingResultDetail,
    pub(super) deadline: DeadlineDetail,
    pub(super) source_event: SourceEventReference,
    pub(super) capture_digest: Sha256Digest,
}

impl HearingDerivedDeadlineCreation {
    pub fn draft(&self) -> &HearingDerivedDeadlineDraft {
        &self.draft
    }
    pub fn result(&self) -> &HearingResultDetail {
        &self.result
    }
    pub fn deadline(&self) -> &DeadlineDetail {
        &self.deadline
    }
    pub const fn source_event(&self) -> SourceEventReference {
        self.source_event
    }
    pub const fn capture_digest(&self) -> Sha256Digest {
        self.capture_digest
    }
}

/// HRDC1 connects the prospective review, exact event and final ordinary receipts.
/// The verified deadline capture commits the complete recorded source evidence.
pub fn hearing_derived_deadline_capture_bytes(
    creation: &HearingDerivedDeadlineCreation,
) -> Result<Vec<u8>, ApplicationError> {
    let mut bytes = b"HRDC1".to_vec();
    bytes.extend_from_slice(creation.draft.review_digest().as_bytes());
    let result = &creation.result.snapshot;
    bytes.extend_from_slice(result.case_id.as_uuid().as_bytes());
    bytes.extend_from_slice(result.hearing_id.as_uuid().as_bytes());
    bytes.extend_from_slice(result.id.as_uuid().as_bytes());
    bytes.extend_from_slice(&result.revision.get().to_be_bytes());
    bytes.extend_from_slice(result.receipt.operation_id.as_uuid().as_bytes());
    bytes.extend_from_slice(result.values_digest.as_bytes());
    bytes.extend_from_slice(result.receipt.submission_digest.as_bytes());
    let event = creation.source_event;
    bytes.extend_from_slice(&event.sequence.to_be_bytes());
    bytes.push(event.family as u8);
    bytes.extend_from_slice(event.source_id.as_bytes());
    bytes.extend_from_slice(&event.revision.to_be_bytes());
    bytes.extend_from_slice(event.operation_id.as_bytes());
    // Event scope is required to match the result encoded above by finalization.
    let deadline = &creation.deadline;
    let submission = deadline_record_submission_bytes(deadline)?;
    bytes.extend_from_slice(&(submission.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&submission);
    bytes.extend_from_slice(deadline.receipt.submission_digest.as_bytes());
    bytes.extend_from_slice(deadline.receipt.capture_digest.as_bytes());
    bytes.extend_from_slice(&deadline.recorded_at.unix_timestamp_nanos().to_be_bytes());
    bytes.extend_from_slice(&deadline.recorded_at.offset().whole_seconds().to_be_bytes());
    Ok(bytes)
}
