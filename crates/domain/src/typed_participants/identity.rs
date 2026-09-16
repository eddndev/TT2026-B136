use crate::{crypto::Sha256Digest, participants::ParticipantRevision, DomainError};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CaseSubjectId(Uuid);
impl CaseSubjectId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }
    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}
impl Default for CaseSubjectId {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Display for CaseSubjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubjectRevision(u32);
impl SubjectRevision {
    pub const fn initial() -> Self {
        Self(1)
    }
    pub fn new(value: u32) -> Result<Self, DomainError> {
        if value == 0 {
            return Err(DomainError::InvalidSubjectRevision);
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u32 {
        self.0
    }
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectExpectation {
    Absent,
    Revision(SubjectRevision),
}
impl SubjectExpectation {
    pub const fn get(self) -> u32 {
        match self {
            Self::Absent => 0,
            Self::Revision(v) => v.get(),
        }
    }
    pub const fn next(self) -> Option<SubjectRevision> {
        match self {
            Self::Absent => Some(SubjectRevision::initial()),
            Self::Revision(v) => v.next(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantExpectation {
    Absent,
    Revision(ParticipantRevision),
}
impl ParticipantExpectation {
    pub const fn get(self) -> u32 {
        match self {
            Self::Absent => 0,
            Self::Revision(v) => v.get(),
        }
    }
    pub const fn next(self) -> Option<ParticipantRevision> {
        match self {
            Self::Absent => Some(ParticipantRevision::initial()),
            Self::Revision(v) => v.next(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectRevisionRef {
    pub id: CaseSubjectId,
    pub revision: SubjectRevision,
    pub values_digest: Sha256Digest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectKind {
    NaturalPerson,
    InstitutionalBody,
}
impl SubjectKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NaturalPerson => "natural_person",
            Self::InstitutionalBody => "institutional_body",
        }
    }
}
