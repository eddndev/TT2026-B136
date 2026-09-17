use super::ApiError;
use application::{deadline_profiles::DeadlineProfileError as P, ApplicationError as A};
use axum::http::StatusCode;
pub(super) fn map(error: A) -> Result<ApiError, A> {
    let (status, code) = match &error {
        A::DeadlineProfile(P::NotFound) => (StatusCode::NOT_FOUND, "deadline_profile_not_found"),
        A::DeadlineProfile(P::RevisionConflict) => {
            (StatusCode::CONFLICT, "deadline_profile_revision_conflict")
        }
        A::DeadlineProfile(P::OperationConflict) => {
            (StatusCode::CONFLICT, "deadline_profile_operation_conflict")
        }
        A::DeadlineProfile(P::Retired) => (StatusCode::CONFLICT, "deadline_profile_retired"),
        A::DeadlineProfile(P::RevisionExhausted) => {
            (StatusCode::CONFLICT, "deadline_profile_revision_exhausted")
        }
        A::DeadlineProfile(P::ScopeChangeForbidden) => (
            StatusCode::CONFLICT,
            "deadline_profile_scope_change_forbidden",
        ),
        A::DeadlineProfile(P::SubmissionMismatch) => {
            (StatusCode::CONFLICT, "deadline_profile_submission_mismatch")
        }
        A::DeadlineProfile(P::Invalid(_)) => {
            (StatusCode::UNPROCESSABLE_ENTITY, "invalid_deadline_profile")
        }
        A::DeadlineProfile(P::ExampleMismatch(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "deadline_profile_example_mismatch",
        ),
        A::DeadlineProfile(P::NoSuccessfulExample) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "deadline_profile_no_successful_example",
        ),
        A::DeadlineProfile(P::StoredInconsistent(_)) => return Ok(ApiError::internal()),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: error.to_string(),
    })
}
