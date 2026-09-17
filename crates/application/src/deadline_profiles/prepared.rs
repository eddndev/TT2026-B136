use super::*;
use domain::{crypto::Sha256Digest, identity::UserId};
/// Only the application service creates validated commits. No timestamp is reserved.
pub struct PreparedDeadlineProfileChange {
    pub(super) actor: UserId,
    pub(super) collection: DeadlineProfileCollection,
    pub(super) algorithm: DeadlineProfileAlgorithm,
    pub(super) command: DeadlineProfileCommand,
    pub(super) preparation: DeadlineProfilePreparation,
    pub(super) definition: DeadlineProfileDefinition,
    pub(super) definition_digest: Sha256Digest,
    pub(super) submission_digest: Sha256Digest,
}
impl PreparedDeadlineProfileChange {
    pub const fn collection(&self) -> DeadlineProfileCollection {
        self.collection
    }
    pub const fn algorithm(&self) -> DeadlineProfileAlgorithm {
        self.algorithm
    }
    pub const fn actor(&self) -> UserId {
        self.actor
    }
    pub fn command(&self) -> &DeadlineProfileCommand {
        &self.command
    }
    pub fn preparation(&self) -> &DeadlineProfilePreparation {
        &self.preparation
    }
    pub fn definition(&self) -> &DeadlineProfileDefinition {
        &self.definition
    }
    pub const fn definition_digest(&self) -> Sha256Digest {
        self.definition_digest
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
}
