use super::{DirectoryStatus, ParticipantId, ParticipantRevision};
use crate::ApplicationError;
use domain::typed_participants::ParticipantKind;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantProfileFilter {
    #[default]
    All,
    Manual,
    Typed,
}

/// Filtering by organizational state; no procedural status is inferred.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantStatusFilter {
    #[default]
    Active,
    Archived,
    All,
}

impl ParticipantStatusFilter {
    pub const fn directory_status(self) -> Option<DirectoryStatus> {
        match self {
            Self::Active => Some(DirectoryStatus::Active),
            Self::Archived => Some(DirectoryStatus::Archived),
            Self::All => None,
        }
    }
}

/// Literal name substring and exact manual role, applied before UUID pagination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantQuery {
    limit: u32,
    after_id: Option<ParticipantId>,
    name: Option<String>,
    procedural_role: Option<String>,
    status: ParticipantStatusFilter,
    kind: Option<ParticipantKind>,
    profile: ParticipantProfileFilter,
}

impl ParticipantQuery {
    pub fn new(
        limit: u32,
        after_id: Option<ParticipantId>,
        name: Option<&str>,
        procedural_role: Option<&str>,
        status: ParticipantStatusFilter,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        Ok(Self {
            limit,
            after_id,
            name: normalize_filter(name, "name", 200)?,
            procedural_role: normalize_filter(procedural_role, "procedural_role", 80)?,
            status,
            kind: None,
            profile: ParticipantProfileFilter::All,
        })
    }

    pub fn with_profile_filter(
        mut self,
        kind: Option<ParticipantKind>,
        profile: ParticipantProfileFilter,
    ) -> Self {
        self.kind = kind;
        self.profile = profile;
        self
    }
    pub const fn kind(&self) -> Option<ParticipantKind> {
        self.kind
    }
    pub const fn profile(&self) -> ParticipantProfileFilter {
        self.profile
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn after_id(&self) -> Option<ParticipantId> {
        self.after_id
    }
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
    pub fn procedural_role(&self) -> Option<&str> {
        self.procedural_role.as_deref()
    }
    pub const fn status(&self) -> ParticipantStatusFilter {
        self.status
    }
}

/// Descending immutable revision history with an exclusive boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticipantHistoryQuery {
    limit: u32,
    before_revision: Option<ParticipantRevision>,
}

impl ParticipantHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        let before_revision = before_revision
            .map(ParticipantRevision::new)
            .transpose()
            .map_err(|error| ApplicationError::InvalidInput(error.to_string()))?;
        Ok(Self {
            limit,
            before_revision,
        })
    }

    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn before_revision(&self) -> Option<ParticipantRevision> {
        self.before_revision
    }
}

fn validate_limit(limit: u32) -> Result<(), ApplicationError> {
    if !(1..=100).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "participant limit must be between 1 and 100".into(),
        ));
    }
    Ok(())
}

fn normalize_filter(
    value: Option<&str>,
    field: &str,
    maximum: usize,
) -> Result<Option<String>, ApplicationError> {
    let Some(value) = value else { return Ok(None) };
    if value.chars().any(char::is_control) {
        return Err(ApplicationError::InvalidInput(format!(
            "participant {field} filter must not contain controls"
        )));
    }
    let value = value.trim();
    if value.chars().count() > maximum {
        return Err(ApplicationError::InvalidInput(format!(
            "participant {field} filter exceeds {maximum} characters"
        )));
    }
    Ok((!value.is_empty()).then(|| value.to_owned()))
}
