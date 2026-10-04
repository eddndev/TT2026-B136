use super::*;
use crate::{
    deadline_reevaluation::SourceEventReference, deadlines::DeadlineDetail,
    hearing_results::HearingResultDetail,
};
use domain::crypto::DocumentHasher;

/// Untrusted historical fields resolved from exact stored revisions and their origin.
/// Captured authority is not current access authorization or proof of persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineEvidence {
    pub actor: Principal,
    pub command: HearingDerivedDeadlineCommand,
    pub material: HearingDerivedDeadlineMaterial,
    pub result: HearingResultDetail,
    pub deadline: DeadlineDetail,
    pub source_event: SourceEventReference,
    pub review_digest: Sha256Digest,
    pub capture_digest: Sha256Digest,
}

/// Verified original evidence, without constructing a new calculated draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HearingDerivedDeadlineRecord {
    evidence: HearingDerivedDeadlineEvidence,
    review_bytes: Vec<u8>,
    capture_bytes: Vec<u8>,
}

impl HearingDerivedDeadlineRecord {
    pub fn evidence(&self) -> &HearingDerivedDeadlineEvidence {
        &self.evidence
    }
    pub fn review_bytes(&self) -> &[u8] {
        &self.review_bytes
    }
    pub fn capture_bytes(&self) -> &[u8] {
        &self.capture_bytes
    }
}

/// Verify original receipts, selected inputs and captured observations. This never
/// evaluates deadline arithmetic or substitutes today's dependency heads or actor.
/// Storage must also verify the event, origin and audit chain, and authorize reads.
pub fn restore_hearing_derived_deadline(
    hasher: &dyn DocumentHasher,
    evidence: HearingDerivedDeadlineEvidence,
) -> Result<HearingDerivedDeadlineRecord, ApplicationError> {
    history_validation::validate(hasher, &evidence)?;
    let review_bytes = canonical::review_bytes(
        hasher,
        &evidence.actor,
        &evidence.command,
        &evidence.material,
        &evidence.deadline.calculation.result,
    )?;
    let capture_bytes = capture::capture_bytes(
        evidence.review_digest,
        &evidence.result,
        &evidence.deadline,
        evidence.source_event,
    )?;
    if hasher.hash_bytes(&review_bytes) != evidence.review_digest
        || hasher.hash_bytes(&capture_bytes) != evidence.capture_digest
    {
        return Err(DeadlineError::SubmissionMismatch.into());
    }
    Ok(HearingDerivedDeadlineRecord {
        evidence,
        review_bytes,
        capture_bytes,
    })
}
