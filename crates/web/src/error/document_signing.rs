use super::ApiError;
use application::{document_signing::SigningWorkflowError as Error, ApplicationError};
use axum::http::StatusCode;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let ApplicationError::DocumentSigning(error) = error else {
        return Err(error);
    };
    let (status, code, message) = match error {
        Error::Input => (
            StatusCode::BAD_REQUEST,
            "invalid_signing_request",
            "invalid individual signing request",
        ),
        Error::AccountChanged => (
            StatusCode::CONFLICT,
            "signing_account_changed",
            "signing account changed",
        ),
        Error::TrustChanged => (
            StatusCode::CONFLICT,
            "signing_trust_changed",
            "signing trust changed",
        ),
        Error::Conflict => (
            StatusCode::CONFLICT,
            "signing_conflict",
            "signing intention conflicts",
        ),
        Error::InactiveBinding => (
            StatusCode::CONFLICT,
            "signing_binding_inactive",
            "signing binding is inactive",
        ),
        Error::NotFound => (
            StatusCode::NOT_FOUND,
            "signing_not_found",
            "signing evidence was not found",
        ),
        Error::TrustUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            "signing_trust_unavailable",
            "signing trust is unavailable",
        ),
        Error::Credential(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "signing_credential_rejected",
            "signing credential verification rejected",
        ),
        Error::Inconsistent => return Ok(ApiError::internal()),
    };
    Ok(ApiError {
        status,
        code,
        message: message.into(),
    })
}
