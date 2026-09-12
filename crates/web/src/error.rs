//! Stable HTTP error mapping for application failures.

use application::ApplicationError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use domain::DomainError;
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn invalid_header(name: &'static str) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: "missing_header",
            message: format!("required header is missing or invalid: {name}"),
        }
    }

    pub fn invalid_document_id() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_document_id",
            message: "document id must be a uuid".to_string(),
        }
    }

    pub fn invalid_case_id() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_case_id",
            message: "case id must be a uuid".to_string(),
        }
    }

    pub fn invalid_user_id() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_user_id",
            message: "user id must be a uuid".to_string(),
        }
    }

    pub fn invalid_response_header() -> Self {
        Self::internal()
    }

    pub(crate) fn busy() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "server_busy",
            message: "server is at capacity; retry later".to_string(),
        }
    }

    pub(crate) fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: "internal application error".to_string(),
        }
    }
}

impl From<ApplicationError> for ApiError {
    fn from(error: ApplicationError) -> Self {
        match error {
            ApplicationError::BootstrapClosed => Self {
                status: StatusCode::CONFLICT,
                code: "bootstrap_closed",
                message: error.to_string(),
            },
            ApplicationError::UserAlreadyExists => Self {
                status: StatusCode::CONFLICT,
                code: "user_already_exists",
                message: error.to_string(),
            },
            ApplicationError::InvalidCredentials => Self {
                status: StatusCode::UNAUTHORIZED,
                code: "invalid_credentials",
                message: error.to_string(),
            },
            ApplicationError::AccountLocked => Self {
                status: StatusCode::TOO_MANY_REQUESTS,
                code: "account_locked",
                message: error.to_string(),
            },
            ApplicationError::MfaRejected => Self {
                status: StatusCode::UNAUTHORIZED,
                code: "mfa_rejected",
                message: error.to_string(),
            },
            ApplicationError::InvalidSession => Self {
                status: StatusCode::UNAUTHORIZED,
                code: "invalid_session",
                message: error.to_string(),
            },
            ApplicationError::PermissionDenied => Self {
                status: StatusCode::FORBIDDEN,
                code: "permission_denied",
                message: error.to_string(),
            },
            ApplicationError::ConcurrentModification => Self {
                status: StatusCode::CONFLICT,
                code: "concurrent_modification",
                message: error.to_string(),
            },
            ApplicationError::CaseNotFound => Self {
                status: StatusCode::NOT_FOUND,
                code: "case_not_found",
                message: error.to_string(),
            },
            ApplicationError::UserNotFound => Self {
                status: StatusCode::NOT_FOUND,
                code: "user_not_found",
                message: error.to_string(),
            },
            ApplicationError::DocumentNotFound(message) => Self {
                status: StatusCode::NOT_FOUND,
                code: "document_not_found",
                message,
            },
            ApplicationError::DocumentAlreadyExists(message) => Self {
                status: StatusCode::CONFLICT,
                code: "document_already_exists",
                message,
            },
            ApplicationError::DocumentAlreadySealed(message) => Self {
                status: StatusCode::CONFLICT,
                code: "document_already_sealed",
                message,
            },
            ApplicationError::DocumentNotSealed(message) => Self {
                status: StatusCode::CONFLICT,
                code: "document_not_sealed",
                message,
            },
            ApplicationError::InvalidInput(message) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_input",
                message,
            },
            ApplicationError::Domain(DomainError::InvalidArchiveEntryName { name }) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_document_name",
                message: format!("document name is not archive safe: {name}"),
            },
            ApplicationError::Domain(DomainError::InvalidCaseMetadata { field, reason }) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_case_metadata",
                message: format!("invalid case {field}: {reason}"),
            },
            _ => Self::internal(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code,
                message: self.message,
            },
        };
        (self.status, Json(body)).into_response()
    }
}
