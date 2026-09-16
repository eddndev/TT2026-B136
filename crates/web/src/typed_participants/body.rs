use crate::error::ApiError;
use axum::extract::Request;
use serde::de::DeserializeOwned;

pub(super) async fn json<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    crate::request::json::read(
        request,
        256 * 1024,
        "typed_participant_body_too_large",
        "typed participant JSON exceeds 256 KiB",
    )
    .await
}
