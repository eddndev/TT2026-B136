use super::*;
use domain::{crypto::Sha256Digest, identity::UserId};
/// Only the application service creates validated commits. No timestamp is reserved.
pub struct PreparedJudicialCalendarChange {
    pub(super) actor: UserId,
    pub(super) command: JudicialCalendarCommand,
    pub(super) preparation: JudicialCalendarPreparation,
    pub(super) values: JudicialCalendarValues,
    pub(super) values_digest: Sha256Digest,
    pub(super) submission_digest: Sha256Digest,
}
impl PreparedJudicialCalendarChange {
    pub const fn actor(&self) -> UserId {
        self.actor
    }
    pub fn command(&self) -> &JudicialCalendarCommand {
        &self.command
    }
    pub fn preparation(&self) -> &JudicialCalendarPreparation {
        &self.preparation
    }
    pub fn values(&self) -> &JudicialCalendarValues {
        &self.values
    }
    pub const fn values_digest(&self) -> Sha256Digest {
        self.values_digest
    }
    pub const fn submission_digest(&self) -> Sha256Digest {
        self.submission_digest
    }
}
