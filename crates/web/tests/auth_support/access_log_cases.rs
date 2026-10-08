use super::{json, router};
use axum::{body::Body, http::Request};
use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn access_logs_correlate_success_rejection_and_invalid_input_without_secrets() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let mut ids = Vec::new();
    for (method, body, status, result, reason) in [
        (
            "totp",
            r#"{"challenge_token":"challenge-token","code":"123456"}"#,
            200,
            "accepted",
            "accepted",
        ),
        (
            "totp",
            r#"{"challenge_token":"private-challenge","code":"654321"}"#,
            401,
            "rejected",
            "rejection_unspecified",
        ),
        (
            "totp",
            r#"{"challenge_token":"private-challenge","code":false}"#,
            422,
            "rejected",
            "invalid_request",
        ),
        (
            "recovery",
            r#"{"challenge_token":"challenge-token","code":"RECOVERY-0"}"#,
            200,
            "accepted",
            "accepted",
        ),
        (
            "recovery",
            r#"{"challenge_token":"private-challenge","code":"RECOVERY-PRIVATE"}"#,
            401,
            "rejected",
            "rejection_unspecified",
        ),
        (
            "totp",
            r#"{"challenge_token":"internal-failure","code":"PRIVATE-CODE"}"#,
            500,
            "error",
            "operational_error",
        ),
    ] {
        let response = router()
            .oneshot(
                Request::post(format!("/api/v1/auth/mfa/{method}"))
                    .header("content-type", "application/json")
                    .header("x-request-id", "untrusted-client-value")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status().as_u16(), status);
        let id = response
            .headers()
            .get("x-request-id")
            .expect("request identifier")
            .to_str()
            .unwrap()
            .to_string();
        uuid::Uuid::parse_str(&id).unwrap();
        assert!(!ids.contains(&id));
        ids.push(id.clone());
        if status == 401 {
            let public = json(response).await;
            assert_eq!(public["error"]["code"], "mfa_rejected");
            assert!(public["error"].get("reason").is_none());
        }
        let output = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
        let events: Vec<serde_json::Value> = output
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(events.len(), ids.len());
        let fields = &events.last().unwrap()["fields"];
        assert_eq!(fields["request_id"], id);
        assert_eq!(fields["result"], result);
        assert!(fields["timestamp_unix_ms"].as_i64().unwrap() > 0);
        assert_eq!(fields["reason"], reason);
        assert_eq!(fields["method"], method);
        assert_eq!(fields.as_object().unwrap().len(), 7);
        if status == 200 {
            assert_eq!(fields["user_id"], "00000000-0000-0000-0000-000000000009");
        }
        for value in fields.as_object().unwrap().values() {
            assert_ne!(value.as_str(), Some("123456"));
            assert_ne!(value.as_str(), Some("654321"));
        }
        for secret in [
            "challenge-token",
            "private-challenge",
            "owner-token",
            "untrusted-client-value",
            "PRIVATE-CODE",
            "RECOVERY-PRIVATE",
            "RECOVERY-0",
            "internal-failure",
            "PRIVATE-ERROR-TOKEN",
            "BASE32SECRET",
            "correct horse battery",
        ] {
            assert!(!output.contains(secret), "secret leaked into access log");
        }
    }
}
