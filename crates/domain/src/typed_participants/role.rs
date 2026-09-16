use super::{
    text::optional, ParticipantEvidenceLocator, ParticipantKind, ParticipantProfile,
    ParticipantText, SubjectRevisionRef,
};
use crate::{participants::DirectoryStatus, DomainError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantRoleValues {
    organization: Option<ParticipantText<200>>,
    legal_status: Option<ParticipantText<160>>,
    profile: ParticipantProfile,
    role_support: ParticipantEvidenceLocator,
}
impl ParticipantRoleValues {
    pub fn new(
        organization: Option<&str>,
        legal_status: Option<&str>,
        profile: ParticipantProfile,
        role_support: ParticipantEvidenceLocator,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            organization: optional(organization)?,
            legal_status: optional(legal_status)?,
            profile,
            role_support,
        })
    }
    pub fn organization(&self) -> Option<&str> {
        self.organization.as_ref().map(ParticipantText::as_str)
    }
    pub fn legal_status(&self) -> Option<&str> {
        self.legal_status.as_ref().map(ParticipantText::as_str)
    }
    pub fn profile(&self) -> &ParticipantProfile {
        &self.profile
    }
    pub fn role_support(&self) -> &ParticipantEvidenceLocator {
        &self.role_support
    }
    pub const fn kind(&self) -> ParticipantKind {
        self.profile.kind()
    }
    pub fn supports(&self) -> Vec<&ParticipantEvidenceLocator> {
        let mut refs = vec![&self.role_support];
        if let ParticipantProfile::Victim(value) = &self.profile {
            if let super::DeclaredContact::Documented(s) = value.contact() {
                refs.push(s);
            }
            if let super::DeclaredProtection::Documented(s) = value.protection() {
                refs.push(s);
            }
        }
        refs
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedParticipantValues {
    subject: SubjectRevisionRef,
    directory_status: DirectoryStatus,
    role: ParticipantRoleValues,
}
impl TypedParticipantValues {
    pub fn new(
        subject: SubjectRevisionRef,
        directory_status: DirectoryStatus,
        role: ParticipantRoleValues,
    ) -> Self {
        Self {
            subject,
            directory_status,
            role,
        }
    }
    pub const fn subject(&self) -> SubjectRevisionRef {
        self.subject
    }
    pub const fn directory_status(&self) -> DirectoryStatus {
        self.directory_status
    }
    pub fn role(&self) -> &ParticipantRoleValues {
        &self.role
    }
    pub const fn kind(&self) -> ParticipantKind {
        self.role.kind()
    }
    pub fn with_directory_status(&self, directory_status: DirectoryStatus) -> Self {
        Self {
            directory_status,
            ..self.clone()
        }
    }
}
