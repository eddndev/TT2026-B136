use super::*;
use crate::documents::StageDocumentFormat;
use domain::crypto::Sha256Digest;

/// Constructed only by the service after exact support validation and hashing.
pub struct PreparedHearingChange {
    pub(super) command: HearingCommand,
    pub(super) preparation: HearingPreparation,
    pub(super) values: HearingValues,
    pub(super) formats: Vec<StageDocumentFormat>,
    pub(super) submission_digest: Sha256Digest,
    pub(super) values_digest: Sha256Digest,
    pub(super) scheduling_context: HearingSchedulingContext,
}
impl PreparedHearingChange {
    pub const fn values_digest(&self) -> Sha256Digest {
        self.values_digest
    }
    pub const fn scheduling_context(&self) -> HearingSchedulingContext {
        self.scheduling_context
    }
    pub fn command(&self) -> &HearingCommand {
        &self.command
    }
    pub fn preparation(&self) -> &HearingPreparation {
        &self.preparation
    }
    pub fn values(&self) -> &HearingValues {
        &self.values
    }
    pub fn formats(&self) -> &[StageDocumentFormat] {
        &self.formats
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
}
