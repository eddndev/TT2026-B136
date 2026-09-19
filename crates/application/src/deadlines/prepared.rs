use super::*;
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId};
/// Only validated application preparation can be submitted to the persistence port.
pub struct PreparedDeadlineChange {
    pub(super) actor: UserId,
    pub(super) tracked_author: Option<DeadlineActorSnapshot>,
    pub(super) tracking: Option<DeadlineTrackingCapture>,
    pub(super) receipt_version: DeadlineReceiptVersion,
    pub(super) case_id: CaseId,
    pub(super) command: DeadlineCommand,
    pub(super) preparation: DeadlinePreparation,
    pub(super) definition: DeadlineDefinition,
    pub(super) calculation: DeadlineCalculation,
    pub(super) responsible: DeadlineResponsibleSnapshot,
    pub(super) attention: DeadlineAttention,
    pub(super) review_digest: Sha256Digest,
    pub(super) capture_digest: Sha256Digest,
    pub(super) submission_digest: Sha256Digest,
}
impl PreparedDeadlineChange {
    pub fn tracking(&self) -> Option<&DeadlineTrackingCapture> {
        self.tracking.as_ref()
    }
    pub fn tracked_author(&self) -> Option<&DeadlineActorSnapshot> {
        self.tracked_author.as_ref()
    }
    pub fn receipt(&self) -> DeadlineReceipt {
        DeadlineReceipt {
            version: self.receipt_version.clone(),
            operation_id: self.command.operation_id,
            action: self.command.action(),
            expected_revision: self.command.expected_revision(),
            review_digest: self.review_digest,
            capture_digest: self.capture_digest,
            submission_digest: self.submission_digest,
        }
    }
    pub const fn actor(&self) -> UserId {
        self.actor
    }
    pub const fn case_id(&self) -> CaseId {
        self.case_id
    }
    pub fn command(&self) -> &DeadlineCommand {
        &self.command
    }
    pub fn preparation(&self) -> &DeadlinePreparation {
        &self.preparation
    }
    pub fn definition(&self) -> &DeadlineDefinition {
        &self.definition
    }
    pub fn calculation(&self) -> &DeadlineCalculation {
        &self.calculation
    }
    pub fn responsible(&self) -> &DeadlineResponsibleSnapshot {
        &self.responsible
    }
    pub fn attention(&self) -> &DeadlineAttention {
        &self.attention
    }
    pub const fn status(&self) -> DeadlineStatus {
        self.command.status()
    }
    pub const fn review_digest(&self) -> Sha256Digest {
        self.review_digest
    }
    pub const fn capture_digest(&self) -> Sha256Digest {
        self.capture_digest
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
}
