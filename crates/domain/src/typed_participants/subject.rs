use super::{Curp, Declared, ParticipantReason, ParticipantText, SubjectKind};
use crate::{
    crypto::{DocumentVersionRef, Sha256Digest},
    DomainError,
};

/// Exact encrypted evidence and a declared human-readable page or section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantEvidenceLocator {
    reference: DocumentVersionRef,
    digest: Sha256Digest,
    locator: ParticipantText<200>,
}
impl ParticipantEvidenceLocator {
    pub fn new(
        reference: DocumentVersionRef,
        digest: Sha256Digest,
        locator: &str,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            reference,
            digest,
            locator: ParticipantText::new(locator)?,
        })
    }
    pub const fn reference(&self) -> DocumentVersionRef {
        self.reference
    }
    pub const fn digest(&self) -> Sha256Digest {
        self.digest
    }
    pub fn locator(&self) -> &str {
        self.locator.as_str()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepresentedName {
    Known(ParticipantText<200>),
    Unidentified {
        label: ParticipantText<200>,
        reason: ParticipantReason,
    },
}
impl RepresentedName {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Known(name) => name.as_str(),
            Self::Unidentified { label, .. } => label.as_str(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubjectValues {
    NaturalPerson {
        name: RepresentedName,
        curp: Declared<Curp>,
        identity_support: ParticipantEvidenceLocator,
    },
    InstitutionalBody {
        name: ParticipantText<200>,
        institutional_identifier: Declared<ParticipantText<80>>,
        identity_support: ParticipantEvidenceLocator,
    },
}
impl SubjectValues {
    pub fn natural_person(
        name: RepresentedName,
        curp: Declared<Curp>,
        identity_support: ParticipantEvidenceLocator,
    ) -> Self {
        Self::NaturalPerson {
            name,
            curp,
            identity_support,
        }
    }
    pub fn institutional_body(
        name: ParticipantText<200>,
        institutional_identifier: Declared<ParticipantText<80>>,
        identity_support: ParticipantEvidenceLocator,
    ) -> Self {
        Self::InstitutionalBody {
            name,
            institutional_identifier,
            identity_support,
        }
    }
    pub const fn kind(&self) -> SubjectKind {
        match self {
            Self::NaturalPerson { .. } => SubjectKind::NaturalPerson,
            Self::InstitutionalBody { .. } => SubjectKind::InstitutionalBody,
        }
    }
    pub fn display_name(&self) -> &str {
        match self {
            Self::NaturalPerson { name, .. } => name.display_name(),
            Self::InstitutionalBody { name, .. } => name.as_str(),
        }
    }
    pub fn identity_support(&self) -> &ParticipantEvidenceLocator {
        match self {
            Self::NaturalPerson {
                identity_support, ..
            }
            | Self::InstitutionalBody {
                identity_support, ..
            } => identity_support,
        }
    }
    pub fn declared_identifier(&self) -> Option<&str> {
        match self {
            Self::NaturalPerson {
                curp: Declared::Known(value),
                ..
            } => Some(value.as_str()),
            Self::InstitutionalBody {
                institutional_identifier: Declared::Known(value),
                ..
            } => Some(value.as_str()),
            _ => None,
        }
    }
}
