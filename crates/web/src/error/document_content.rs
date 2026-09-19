use super::ApiError;
use application::ApplicationError;
use axum::http::StatusCode;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let (status, code, message) = match error {
        ApplicationError::DocumentContentValidationFailed(_) => (
            StatusCode::CONFLICT,
            "document_content_validation_failed",
            "document content validation failed; no file was released",
        ),
        ApplicationError::DocumentContentTooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "document_content_too_large",
            "document content exceeds the supported read size",
        ),
        ApplicationError::DocumentIntegrityIncidentNotFound(_) => (
            StatusCode::NOT_FOUND,
            "document_integrity_incident_not_found",
            "document integrity incident was not found",
        ),
        ApplicationError::DocumentIntegrityObservationConflict => (
            StatusCode::CONFLICT,
            "document_integrity_observation_conflict",
            "document integrity observation conflicts with a recorded observation",
        ),
        other => return Err(other),
    };
    Ok(ApiError {
        status,
        code,
        message: message.into(),
    })
}
