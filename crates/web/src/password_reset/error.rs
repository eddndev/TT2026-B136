use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub(super) enum ResetError {
    Json,
    Query,
    MediaType,
    TooLarge,
    Unavailable,
    Rejected,
    Password,
    Uncertain,
}

#[derive(Serialize)]
struct Envelope {
    error: Message,
}

#[derive(Serialize)]
struct Message {
    code: &'static str,
    message: &'static str,
}

impl IntoResponse for ResetError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::Json => (
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "invalid JSON object",
            ),
            Self::Query => (
                StatusCode::BAD_REQUEST,
                "invalid_password_reset_request",
                "password reset requests do not accept a query string",
            ),
            Self::MediaType => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "one JSON content type is required",
            ),
            Self::TooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "password_reset_body_too_large",
                "password reset request body exceeds 16384 bytes",
            ),
            Self::Unavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "password_reset_unavailable",
                "password reset is unavailable",
            ),
            Self::Rejected => (
                StatusCode::BAD_REQUEST,
                "password_reset_rejected",
                "reset link cannot be used; request another link",
            ),
            Self::Password => (
                StatusCode::BAD_REQUEST,
                "invalid_password",
                "password must contain between 12 and 1024 bytes",
            ),
            Self::Uncertain => (
                StatusCode::SERVICE_UNAVAILABLE,
                "password_reset_uncertain",
                "password reset outcome could not be confirmed",
            ),
        };
        (
            status,
            Json(Envelope {
                error: Message { code, message },
            }),
        )
            .into_response()
    }
}
