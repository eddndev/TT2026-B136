use super::ApiError;
use application::{ApplicationError, PortFailureKind};
use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};

#[tokio::test]
async fn classified_port_errors_keep_the_opaque_internal_http_response() {
    let failures = [
        ApplicationError::Port("private database diagnostic".into()),
        ApplicationError::ClassifiedPort {
            kind: PortFailureKind::Busy,
            message: "private lock diagnostic".into(),
        },
        ApplicationError::ClassifiedPort {
            kind: PortFailureKind::Interrupted,
            message: "private transaction diagnostic".into(),
        },
        ApplicationError::ClassifiedPort {
            kind: PortFailureKind::Unavailable,
            message: "private connection diagnostic".into(),
        },
    ];
    for failure in failures {
        let response = ApiError::from(failure).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()["content-type"], "application/json");
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        assert_eq!(
            &body[..],
            br#"{"error":{"code":"internal_error","message":"internal application error"}}"#
        );
    }
}
