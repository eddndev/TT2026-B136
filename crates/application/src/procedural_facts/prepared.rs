use super::*;
use domain::crypto::Sha256Digest;

/// Only the service can construct this after validating the complete direct batch.
pub struct PreparedFactChange {
    pub(super) command: ProceduralFactCommand,
    pub(super) preparation: FactPreparation,
    pub(super) values: ProceduralFactValues,
    pub(super) values_digest: Sha256Digest,
    pub(super) sources: FactSources,
    pub(super) sources_digest: Sha256Digest,
    pub(super) submission_digest: Sha256Digest,
}
impl PreparedFactChange {
    pub fn command(&self) -> &ProceduralFactCommand {
        &self.command
    }
    pub fn preparation(&self) -> &FactPreparation {
        &self.preparation
    }
    pub fn values(&self) -> &ProceduralFactValues {
        &self.values
    }
    pub const fn values_digest(&self) -> Sha256Digest {
        self.values_digest
    }
    pub fn sources(&self) -> &FactSources {
        &self.sources
    }
    pub const fn sources_digest(&self) -> Sha256Digest {
        self.sources_digest
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
}
