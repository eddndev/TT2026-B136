//! Stable HTTP error mapping for application failures.

use application::ApplicationError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use domain::DomainError;
use serde::Serialize;

mod hearing;
mod hearing_result;
mod judicial_calendar;
mod procedural_fact;
mod stage;
mod typed_participant;

#[cfg(test)]
mod hearing_tests;
#[cfg(test)]
mod procedural_fact_tests;
#[cfg(test)]
mod stage_tests;

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
    pub(crate) fn invalid_body(code: &'static str, message: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.to_owned(),
        }
    }

    pub(crate) fn payload_too_large(code: &'static str, message: &str) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code,
            message: message.to_owned(),
        }
    }
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

    pub fn invalid_document_version() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_document_version",
            message: "document version must be an integer between 1 and 4294967295".into(),
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
        let error = match procedural_fact::map(error) {
            Ok(mapped) => return mapped,
            Err(error) => error,
        };
        let error = match judicial_calendar::map(error) {
            Ok(mapped) => return mapped,
            Err(error) => error,
        };
        let error = match hearing_result::map(error) {
            Ok(mapped) => return mapped,
            Err(error) => error,
        };
        let error = match hearing::map(error) {
            Ok(mapped) => return mapped,
            Err(error) => error,
        };
        let error = match stage::map(error) {
            Ok(mapped) => return mapped,
            Err(error) => error,
        };
        let error = match typed_participant::map(error) {
            Ok(mapped) => return mapped,
            Err(error) => error,
        };
        match error {
            ApplicationError::StageSupportTooLarge => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "stage_support_too_large",
                message: error.to_string(),
            },
            ApplicationError::StageSupportFormatRejected => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "stage_support_format_rejected",
                message: error.to_string(),
            },
            ApplicationError::StageSupportValidationLimit => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "stage_support_validation_limit",
                message: error.to_string(),
            },
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
            ApplicationError::CaseRevisionConflict => Self {
                status: StatusCode::CONFLICT,
                code: "case_revision_conflict",
                message: error.to_string(),
            },
            ApplicationError::CaseRevisionExhausted => Self {
                status: StatusCode::CONFLICT,
                code: "case_revision_exhausted",
                message: error.to_string(),
            },
            ApplicationError::CaseClosed => Self {
                status: StatusCode::CONFLICT,
                code: "case_closed",
                message: error.to_string(),
            },
            ApplicationError::CaseIdentifierConflict => Self {
                status: StatusCode::CONFLICT,
                code: "case_identifier_conflict",
                message: error.to_string(),
            },
            ApplicationError::CaseProfileRequired => Self {
                status: StatusCode::CONFLICT,
                code: "case_profile_required",
                message: error.to_string(),
            },
            ApplicationError::Domain(DomainError::InvalidPenalCaseProfile { .. }) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_penal_case_profile",
                message: error.to_string(),
            },
            ApplicationError::Domain(DomainError::InvalidCaseRevision) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_case_revision",
                message: error.to_string(),
            },
            ApplicationError::Domain(DomainError::InvalidCaseStageRevision) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_case_stage_revision",
                message: error.to_string(),
            },
            ApplicationError::Domain(DomainError::InvalidCaseAdministrativeStatus) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_case_administrative_status",
                message: error.to_string(),
            },
            ApplicationError::ParticipantNotFound => Self {
                status: StatusCode::NOT_FOUND,
                code: "participant_not_found",
                message: error.to_string(),
            },
            ApplicationError::ParticipantRevisionConflict => Self {
                status: StatusCode::CONFLICT,
                code: "participant_revision_conflict",
                message: error.to_string(),
            },
            ApplicationError::ParticipantRevisionExhausted => Self {
                status: StatusCode::CONFLICT,
                code: "participant_revision_exhausted",
                message: error.to_string(),
            },
            ApplicationError::Domain(DomainError::InvalidParticipantValues { field, reason }) => {
                Self {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    code: "invalid_participant_values",
                    message: format!("invalid participant {field}: {reason}"),
                }
            }
            ApplicationError::Domain(DomainError::InvalidParticipantRevision) => Self {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                code: "invalid_participant_revision",
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
            ApplicationError::DocumentMetadataConflict => Self {
                status: StatusCode::CONFLICT,
                code: "document_metadata_conflict",
                message: error.to_string(),
            },
            ApplicationError::DocumentMetadataRevisionExhausted => Self {
                status: StatusCode::CONFLICT,
                code: "document_metadata_revision_exhausted",
                message: error.to_string(),
            },
            ApplicationError::Domain(DomainError::InvalidDocumentMetadata { field, reason }) => {
                Self {
                    status: StatusCode::UNPROCESSABLE_ENTITY,
                    code: "invalid_document_metadata",
                    message: format!("invalid document {field}: {reason}"),
                }
            }
            ApplicationError::DocumentVersionConflict => Self {
                status: StatusCode::CONFLICT,
                code: "document_version_conflict",
                message: error.to_string(),
            },
            ApplicationError::DocumentVersionRequired => Self {
                status: StatusCode::CONFLICT,
                code: "document_version_required",
                message: error.to_string(),
            },
            ApplicationError::DocumentVersionExhausted => Self {
                status: StatusCode::CONFLICT,
                code: "document_version_exhausted",
                message: error.to_string(),
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
