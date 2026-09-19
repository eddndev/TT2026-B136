use super::ApiError;
use application::{alerts::AlertError, ApplicationError};
use axum::http::StatusCode;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let (status, code, message) = match &error {
        ApplicationError::Alert(AlertError::NotFound) => {
            (StatusCode::NOT_FOUND, "alert_not_found", "alert not found")
        }
        ApplicationError::Alert(AlertError::RevisionConflict) => (
            StatusCode::CONFLICT,
            "alert_revision_conflict",
            "alert preferences changed",
        ),
        ApplicationError::Alert(AlertError::OperationConflict) => (
            StatusCode::CONFLICT,
            "alert_operation_conflict",
            "alert operation already used",
        ),
        ApplicationError::Alert(AlertError::Invalid(_) | AlertError::Timing(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_alert",
            "invalid alert values",
        ),
        ApplicationError::Alert(AlertError::Stored(_)) => return Ok(ApiError::internal()),
        _ => return Err(error),
    };
    Ok(ApiError {
        status,
        code,
        message: message.into(),
    })
}
