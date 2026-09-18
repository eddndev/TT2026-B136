use super::request;
use crate::error::ApiError;
use application::deadlines::*;
use domain::cases::CaseId;
use serde::Deserialize;
#[derive(Deserialize)]
pub(super) struct RoutePath {
    case_id: String,
    id: Option<String>,
    revision: Option<String>,
}
impl RoutePath {
    pub(super) fn case_id(&self) -> Result<CaseId, ApiError> {
        request::uuid(&self.case_id, "invalid_case_id").map(CaseId::from_uuid)
    }
    pub(super) fn id(&self) -> Result<DeadlineId, ApiError> {
        request::uuid(
            self.id.as_deref().ok_or_else(ApiError::internal)?,
            "invalid_deadline_id",
        )
        .map(DeadlineId::from_uuid)
    }
    pub(super) fn revision(&self) -> Result<DeadlineRevision, ApiError> {
        let invalid = || {
            ApiError::invalid_body(
                "invalid_deadline_revision",
                "revision must be a positive u32",
            )
        };
        let value = request::parse_u32(self.revision.as_deref().ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        DeadlineRevision::new(value).map_err(|_| invalid())
    }
}
pub(super) fn accepts(case_id: CaseId, command: &DeadlineCommand) -> bool {
    match &command.change {
        DeadlineChange::Register { definition } | DeadlineChange::Correct { definition, .. } => {
            definition.input.selection.case_id == case_id
        }
        _ => true,
    }
}
pub(super) fn mismatch() -> ApiError {
    ApiError::invalid_body(
        "deadline_command_mismatch",
        "route, case, deadline and action must agree",
    )
}
