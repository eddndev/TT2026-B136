use application::ApplicationError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::error::ApiError;

pub(super) enum InvitationError {
    Invalid,
    Json,
    MediaType,
    TooLarge,
    Unavailable,
    Rejected,
    Uncertain,
    Api(ApiError),
}

impl InvitationError {
    pub(super) fn owner(error: ApplicationError) -> Self {
        match error {
            ApplicationError::InvalidInput(_) => Self::Invalid,
            other => Self::Api(other.into()),
        }
    }

    pub(super) fn recipient(error: ApplicationError) -> Self {
        match error {
            ApplicationError::InvalidCredentials
            | ApplicationError::MfaRejected
            | ApplicationError::AccountLocked
            | ApplicationError::ConcurrentModification
            | ApplicationError::UserAlreadyExists
            | ApplicationError::UserNotFound
            | ApplicationError::InvalidInput(_) => Self::Rejected,
            _ => Self::Uncertain,
        }
    }
}

impl From<ApiError> for InvitationError {
    fn from(error: ApiError) -> Self {
        Self::Api(error)
    }
}

impl IntoResponse for InvitationError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Api(error) => return error.into_response(),
            Self::Invalid => (
                StatusCode::BAD_REQUEST,
                "invalid_invitation_request",
                "invalid invitation request",
            ),
            Self::Json => (
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "invalid JSON object",
            ),
            Self::MediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "one JSON content type is required",
            ),
            Self::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "invitation_body_too_large",
                "invitation request body exceeds 16384 bytes",
            ),
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "invitation_unavailable",
                "invitations are unavailable",
            ),
            Self::Rejected => (
                StatusCode::BAD_REQUEST,
                "invitation_rejected",
                "invitation request was rejected",
            ),
            Self::Uncertain => (
                StatusCode::SERVICE_UNAVAILABLE,
                "invitation_uncertain",
                "invitation outcome could not be confirmed",
            ),
        };
        (
            status,
            Json(json!({"error":{"code":code,"message":message}})),
        )
            .into_response()
    }
}
