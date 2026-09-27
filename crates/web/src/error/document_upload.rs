use super::ApiError;
use application::{documents::DocumentUploadError, ApplicationError};
use axum::http::StatusCode;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let ApplicationError::DocumentUpload(cause) = error else {
        return Err(error);
    };
    let (status, code) = match cause {
        DocumentUploadError::Unsupported => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_format_unsupported",
        ),
        DocumentUploadError::Invalid => {
            (StatusCode::UNPROCESSABLE_ENTITY, "document_format_invalid")
        }
        DocumentUploadError::Limit => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_validation_limit",
        ),
        DocumentUploadError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "document_validator_unavailable",
        ),
    };
    Ok(ApiError {
        status,
        code,
        message: cause.to_string(),
    })
}
