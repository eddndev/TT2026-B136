use super::ApiError;
use application::{documents::DocumentUploadError, ApplicationError};
use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use serde_json::json;

#[tokio::test]
async fn upload_admission_errors_have_distinct_public_codes_and_safe_messages() {
    for (cause, status, code, message) in [
        (
            DocumentUploadError::Unsupported,
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_format_unsupported",
            "document format is not supported",
        ),
        (
            DocumentUploadError::Invalid,
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_format_invalid",
            "document format is invalid",
        ),
        (
            DocumentUploadError::Limit,
            StatusCode::UNPROCESSABLE_ENTITY,
            "document_validation_limit",
            "document validation exceeded its resource limit",
        ),
        (
            DocumentUploadError::Unavailable,
            StatusCode::SERVICE_UNAVAILABLE,
            "document_validator_unavailable",
            "document validator is unavailable",
        ),
    ] {
        let response = ApiError::from(ApplicationError::DocumentUpload(cause)).into_response();
        assert_eq!(response.status(), status, "{cause:?}");
        assert_eq!(response.headers()["content-type"], "application/json");
        let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, json!({"error":{"code":code,"message":message}}));
    }
}
