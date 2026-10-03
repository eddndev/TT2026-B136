use application::{identity::owner_certificates::OwnerCertificateError as Error, ApplicationError};
use axum::http::StatusCode;

use super::ApiError;

pub(super) fn map(error: ApplicationError) -> Result<ApiError, ApplicationError> {
    let ApplicationError::OwnerCertificate(error) = error else {
        return Err(error);
    };
    let (status, code, message) = match error {
        Error::InvalidInput => (
            StatusCode::BAD_REQUEST,
            "owner_certificate_invalid_input",
            "invalid owner certificate request",
        ),
        Error::NotFound => (
            StatusCode::NOT_FOUND,
            "owner_certificate_not_found",
            "owner certificate binding was not found",
        ),
        Error::AccountChanged
        | Error::TrustUnavailable
        | Error::TrustChanged
        | Error::BindingConflict
        | Error::FingerprintConflict
        | Error::ActiveBinding
        | Error::RevisionConflict => (
            StatusCode::CONFLICT,
            "owner_certificate_conflict",
            "owner certificate state conflicts with the request",
        ),
        Error::MaterialMismatch | Error::Credential(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            "owner_certificate_credential_rejected",
            "owner certificate verification rejected",
        ),
        Error::Inconsistent => return Ok(ApiError::internal()),
    };
    Ok(ApiError {
        status,
        code,
        message: message.into(),
    })
}
