use super::ApiError;
use application::{procedural_resources::ProceduralResourceError as R, ApplicationError as A};
use axum::http::StatusCode;
use domain::DomainError;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::ProceduralResource(R::NotFound) => {
            (StatusCode::NOT_FOUND, "procedural_resource_not_found")
        }
        A::ProceduralResource(R::RevisionConflict) => (
            StatusCode::CONFLICT,
            "procedural_resource_revision_conflict",
        ),
        A::ProceduralResource(R::OperationConflict) => (
            StatusCode::CONFLICT,
            "procedural_resource_operation_conflict",
        ),
        A::ProceduralResource(R::Archived) => {
            (StatusCode::CONFLICT, "procedural_resource_archived")
        }
        A::ProceduralResource(R::StateUnchanged) => {
            (StatusCode::CONFLICT, "procedural_resource_state_unchanged")
        }
        A::ProceduralResource(R::SubmissionMismatch) => (
            StatusCode::CONFLICT,
            "procedural_resource_submission_mismatch",
        ),
        A::Domain(DomainError::InvalidProceduralResource(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_procedural_resource",
        ),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
