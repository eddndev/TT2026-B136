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

// Logging reads only this category; the private message never reaches it.
#[test]
fn opaque_port_errors_name_a_category_for_the_server_log() {
    let message = || "private diagnostic".to_string();
    for (failure, category) in [
        (ApplicationError::Port(message()), "port"),
        (
            ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Busy,
                message: message(),
            },
            "busy",
        ),
        (
            ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Interrupted,
                message: message(),
            },
            "interrupted",
        ),
        (
            ApplicationError::ClassifiedPort {
                kind: PortFailureKind::Unavailable,
                message: message(),
            },
            "unavailable",
        ),
    ] {
        assert_eq!(super::port_log::category(&failure), Some(category));
    }
    let denied = ApplicationError::PermissionDenied;
    assert_eq!(super::port_log::category(&denied), None);
}
