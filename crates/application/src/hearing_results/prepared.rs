use super::*;
use crate::documents::StageDocumentFormat;
use domain::crypto::Sha256Digest;
/// Only the service can construct this after validating the exact proposed support.
pub struct PreparedHearingResultChange {
    pub(super) command: HearingResultCommand,
    pub(super) preparation: HearingResultPreparation,
    pub(super) values: HearingResultValues,
    pub(super) formats: Vec<StageDocumentFormat>,
    pub(super) values_digest: Sha256Digest,
    pub(super) submission_digest: Sha256Digest,
    pub(super) anchor: HearingResultAnchor,
    pub(super) continuation: Option<HearingResultContinuation>,
}
impl PreparedHearingResultChange {
    pub fn command(&self) -> &HearingResultCommand {
        &self.command
    }
    pub fn preparation(&self) -> &HearingResultPreparation {
        &self.preparation
    }
    pub fn values(&self) -> &HearingResultValues {
        &self.values
    }
    pub fn formats(&self) -> &[StageDocumentFormat] {
        &self.formats
    }
    pub const fn values_digest(&self) -> Sha256Digest {
        self.values_digest
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
    pub const fn anchor(&self) -> HearingResultAnchor {
        self.anchor
    }
    pub const fn continuation(&self) -> Option<HearingResultContinuation> {
        self.continuation
    }
}
