use super::ApiError;
use application::{deadlines::DeadlineError as D, ApplicationError as A};
use axum::http::StatusCode;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::Deadline(D::NotFound) => (StatusCode::NOT_FOUND, "deadline_not_found"),
        A::Deadline(D::RevisionConflict) => (StatusCode::CONFLICT, "deadline_revision_conflict"),
        A::Deadline(D::OperationConflict) => (StatusCode::CONFLICT, "deadline_operation_conflict"),
        A::Deadline(D::RevisionExhausted) => (StatusCode::CONFLICT, "deadline_revision_exhausted"),
        A::Deadline(D::Retired) => (StatusCode::CONFLICT, "deadline_retired"),
        A::Deadline(D::ProfileUnavailable) => {
            (StatusCode::CONFLICT, "deadline_profile_unavailable")
        }
        A::Deadline(D::ResponsibleUnavailable) => {
            (StatusCode::CONFLICT, "deadline_responsible_unavailable")
        }
        A::Deadline(D::SubmissionMismatch) => {
            (StatusCode::CONFLICT, "deadline_submission_mismatch")
        }
        A::Deadline(D::Invalid(_)) => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_deadline"),
        A::Deadline(D::StoredInconsistent(_)) => return Ok(ApiError::internal()),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
