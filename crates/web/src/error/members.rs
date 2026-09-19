use super::ApiError;
use application::{members::MemberError, ApplicationError};
use axum::http::StatusCode;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let ApplicationError::Member(error) = error else {
        return Err(error);
    };
    let (status, code, message) = match error {
        MemberError::Invalid(_) => (
            StatusCode::BAD_REQUEST,
            "invalid_input",
            "invalid member input",
        ),
        MemberError::Stored(_) => return Ok(ApiError::internal()),
        MemberError::RevisionConflict => (
            StatusCode::CONFLICT,
            "user_revision_conflict",
            "user revision changed",
        ),
        MemberError::LastActiveOwner => (
            StatusCode::CONFLICT,
            "last_active_owner",
            "at least one active owner must remain",
        ),
        MemberError::AccessVersionExhausted => (
            StatusCode::CONFLICT,
            "user_access_version_exhausted",
            "user access version is exhausted",
        ),
    };
    Ok(ApiError {
        status,
        code,
        message: message.into(),
    })
}
