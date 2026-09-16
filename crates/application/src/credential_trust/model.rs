use crate::ApplicationError;
use domain::clock::OffsetDateTime;
use domain::crypto::CredentialTrustInspection;
use domain::typed_participants::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CredentialTrustRevision(u32);
impl CredentialTrustRevision {
    pub const fn initial() -> Self {
        Self(1)
    }
    pub fn new(value: u32) -> Result<Self, ApplicationError> {
        if value == 0 {
            return Err(ApplicationError::InvalidInput(
                "credential trust revision must be positive".into(),
            ));
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u32 {
        self.0
    }
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialTrustExpectation {
    Absent,
    Revision(CredentialTrustRevision),
}
impl CredentialTrustExpectation {
    pub const fn get(self) -> u32 {
        match self {
            Self::Absent => 0,
            Self::Revision(v) => v.get(),
        }
    }
    pub const fn next(self) -> Option<CredentialTrustRevision> {
        match self {
            Self::Absent => Some(CredentialTrustRevision::initial()),
            Self::Revision(v) => v.next(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialTrustSnapshot {
    pub deployment_id: Uuid,
    pub revision: CredentialTrustRevision,
    pub inspection: CredentialTrustInspection,
    pub published_at: OffsetDateTime,
    pub published_by: String,
}
