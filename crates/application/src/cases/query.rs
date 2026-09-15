use domain::cases::CaseId;

use super::CaseRevision;
use crate::ApplicationError;

/// Expected absence of history differs from an expected positive revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseRevisionExpectation {
    Unrevised,
    Revision(CaseRevision),
}
impl CaseRevisionExpectation {
    pub fn new(value: u32) -> Self {
        match CaseRevision::new(value) {
            Ok(revision) => Self::Revision(revision),
            Err(_) => Self::Unrevised,
        }
    }
    pub const fn get(self) -> u32 {
        match self {
            Self::Unrevised => 0,
            Self::Revision(revision) => revision.get(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CaseStatusFilter {
    #[default]
    Active,
    Closed,
    All,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CaseProfileFilter {
    Complete,
    Pending,
    #[default]
    All,
}

/// Literal title substring and exact identifiers, applied to visible heads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationQuery {
    limit: u32,
    after_id: Option<CaseId>,
    status: CaseStatusFilter,
    profile: CaseProfileFilter,
    title: Option<String>,
    nuc: Option<String>,
    judicial_case_number: Option<String>,
}
impl CaseAdministrationQuery {
    pub fn new(
        limit: u32,
        after_id: Option<CaseId>,
        status: CaseStatusFilter,
        profile: CaseProfileFilter,
        title: Option<&str>,
        nuc: Option<&str>,
        judicial_case_number: Option<&str>,
    ) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        Ok(Self {
            limit,
            after_id,
            status,
            profile,
            title: normalize(title, "title", 200)?,
            nuc: normalize(nuc, "nuc", 100)?,
            judicial_case_number: normalize(judicial_case_number, "judicial_case_number", 100)?,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn after_id(&self) -> Option<CaseId> {
        self.after_id
    }
    pub const fn status(&self) -> CaseStatusFilter {
        self.status
    }
    pub const fn profile(&self) -> CaseProfileFilter {
        self.profile
    }
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    pub fn nuc(&self) -> Option<&str> {
        self.nuc.as_deref()
    }
    pub fn judicial_case_number(&self) -> Option<&str> {
        self.judicial_case_number.as_deref()
    }
}

/// Exclusive positive cursor through descending immutable history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseAdministrationHistoryQuery {
    limit: u32,
    before_revision: Option<CaseRevision>,
}
impl CaseAdministrationHistoryQuery {
    pub fn new(limit: u32, before_revision: Option<u32>) -> Result<Self, ApplicationError> {
        validate_limit(limit)?;
        let before_revision = before_revision
            .map(CaseRevision::new)
            .transpose()
            .map_err(|e| ApplicationError::InvalidInput(e.to_string()))?;
        Ok(Self {
            limit,
            before_revision,
        })
    }
    pub const fn limit(&self) -> u32 {
        self.limit
    }
    pub const fn before_revision(&self) -> Option<CaseRevision> {
        self.before_revision
    }
}

pub(super) fn validate_limit(limit: u32) -> Result<(), ApplicationError> {
    if !(1..=100).contains(&limit) {
        return Err(ApplicationError::InvalidInput(
            "case limit must be between 1 and 100".into(),
        ));
    }
    Ok(())
}
fn normalize(
    value: Option<&str>,
    field: &str,
    maximum: usize,
) -> Result<Option<String>, ApplicationError> {
    let Some(value) = value else { return Ok(None) };
    if value.chars().any(char::is_control) {
        return Err(ApplicationError::InvalidInput(format!(
            "case {field} filter must not contain controls"
        )));
    }
    let value = value.trim();
    if value.chars().count() > maximum {
        return Err(ApplicationError::InvalidInput(format!(
            "case {field} filter exceeds {maximum} characters"
        )));
    }
    Ok((!value.is_empty()).then(|| value.to_owned()))
}
