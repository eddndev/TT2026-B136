use super::ApiError;
use application::{resource_activities::ResourceActivityError as R, ApplicationError as A};
use axum::http::StatusCode;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::ResourceActivity(R::NotFound) => (StatusCode::NOT_FOUND, "resource_activity_not_found"),
        A::ResourceActivity(R::RevisionConflict) => {
            (StatusCode::CONFLICT, "resource_activity_revision_conflict")
        }
        A::ResourceActivity(R::ResourceRevisionConflict) => (
            StatusCode::CONFLICT,
            "resource_activity_resource_revision_conflict",
        ),
        A::ResourceActivity(R::OperationConflict) => {
            (StatusCode::CONFLICT, "resource_activity_operation_conflict")
        }
        A::ResourceActivity(R::ResourceArchived) => {
            (StatusCode::CONFLICT, "resource_activity_resource_archived")
        }
        A::ResourceActivity(R::StateUnchanged) => {
            (StatusCode::CONFLICT, "resource_activity_state_unchanged")
        }
        A::ResourceActivity(R::SourceMismatch) => {
            (StatusCode::CONFLICT, "resource_activity_source_mismatch")
        }
        A::ResourceActivity(R::SubmissionMismatch) => (
            StatusCode::CONFLICT,
            "resource_activity_submission_mismatch",
        ),
        A::ResourceActivity(R::StoredInconsistent(_)) => return Ok(ApiError::internal()),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
