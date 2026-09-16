//! Read the complete bounded entity before deserializing any proposal.

use crate::error::ApiError;
use axum::{body::to_bytes, extract::Request, http::header::CONTENT_TYPE};
use serde::de::DeserializeOwned;
use std::error::Error;

pub(crate) async fn read<T: DeserializeOwned>(
    request: Request,
    limit: usize,
    limit_code: &'static str,
    limit_message: &str,
) -> Result<T, ApiError> {
    let mut types = request.headers().get_all(CONTENT_TYPE).iter();
    let content_type = types.next().and_then(|v| v.to_str().ok());
    if types.next().is_some() || !content_type.is_some_and(is_json) {
        return Err(invalid());
    }
    let bytes = to_bytes(request.into_body(), limit)
        .await
        .map_err(|error| {
            if error
                .source()
                .is_some_and(|e| e.is::<http_body_util::LengthLimitError>())
            {
                ApiError::payload_too_large(limit_code, limit_message)
            } else {
                invalid()
            }
        })?;
    // from_slice checks end-of-input, including content after the first object.
    serde_json::from_slice(&bytes).map_err(|_| invalid())
}
fn is_json(value: &str) -> bool {
    let mime = value.split(';').next().unwrap_or("").trim();
    let mime = mime.to_ascii_lowercase();
    mime == "application/json" || (mime.starts_with("application/") && mime.ends_with("+json"))
}
fn invalid() -> ApiError {
    ApiError::invalid_body("invalid_json", "invalid JSON object")
}
