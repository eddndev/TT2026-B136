use super::ApiError;
use application::members::{
    CaseMemberQuery, MemberSelection, UserAccessChange, UserQuery, UserStatusFilter,
};
use domain::{cases::CaseId, identity::Role};
use serde::Deserialize;

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_input", "invalid member query or access change")
}
fn limit() -> u32 {
    50
}
fn active() -> String {
    "active".into()
}
fn assigned() -> String {
    "assigned".into()
}
fn role(value: &str) -> Result<Role, ApiError> {
    value.parse().map_err(|_| invalid())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DirectoryInput {
    #[serde(default = "limit")]
    limit: u32,
    #[serde(default = "active")]
    status: String,
    role: Option<String>,
    email_prefix: Option<String>,
    cursor: Option<String>,
}
impl DirectoryInput {
    pub(super) fn query(self) -> Result<UserQuery, ApiError> {
        let status = match self.status.as_str() {
            "active" => UserStatusFilter::Active,
            "inactive" => UserStatusFilter::Inactive,
            "all" => UserStatusFilter::All,
            _ => return Err(invalid()),
        };
        UserQuery::new(
            self.limit,
            status,
            self.role.as_deref().map(role).transpose()?,
            self.email_prefix.as_deref(),
            self.cursor.as_deref(),
        )
        .map_err(|error| application::ApplicationError::from(error).into())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssignmentInput {
    #[serde(default = "limit")]
    limit: u32,
    #[serde(default = "assigned")]
    selection: String,
    role: Option<String>,
    email_prefix: Option<String>,
    cursor: Option<String>,
}
impl AssignmentInput {
    pub(super) fn query(self, case_id: CaseId) -> Result<CaseMemberQuery, ApiError> {
        let selection = match self.selection.as_str() {
            "assigned" => MemberSelection::Assigned,
            "available" => MemberSelection::Available,
            _ => return Err(invalid()),
        };
        CaseMemberQuery::new(
            case_id,
            self.limit,
            selection,
            self.role.as_deref().map(role).transpose()?,
            self.email_prefix.as_deref(),
            self.cursor.as_deref(),
        )
        .map_err(|error| application::ApplicationError::from(error).into())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AccessInput {
    expected_revision: String,
    role: String,
    active: bool,
}
impl AccessInput {
    pub(super) fn change(self) -> Result<UserAccessChange, ApiError> {
        let value = self.expected_revision;
        if value.is_empty()
            || value.len() > 19
            || !value.bytes().all(|b| b.is_ascii_digit())
            || (value.len() > 1 && value.starts_with('0'))
        {
            return Err(invalid());
        }
        let revision = value.parse::<u64>().map_err(|_| invalid())?;
        if revision > i64::MAX as u64 {
            return Err(invalid());
        }
        UserAccessChange::new(revision, role(&self.role)?, self.active)
            .map_err(|error| application::ApplicationError::from(error).into())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NoQuery {}
