#[path = "../auth_support/capture.rs"]
mod capture;

use capture::Capture;
use domain::identity::Role;
use serde_json::Value;

use crate::ca_access_support::*;

#[tokio::test]
async fn mfa_logs_report_final_ca_admission_with_no_credentials_or_office_role() {
    let capture = Capture::default();
    let writer = capture.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .without_time()
        .with_writer(move || writer.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);
    let mut secrets = vec![PASSWORD.to_owned(), "RECOVERY-0".to_owned()];
    let mut request_ids = Vec::new();
    for factor in ["totp", "recovery"] {
        for (role, configured_id, status, result, reason) in [
            (Role::Client, id(1), 200, "accepted", "accepted"),
            (
                Role::Owner,
                id(1),
                403,
                "rejected",
                "session_admission_rejected",
            ),
            (
                Role::Client,
                id(2),
                403,
                "rejected",
                "session_admission_rejected",
            ),
        ] {
            let harness = Harness::configured(role, configured_id);
            let challenge = harness.challenge().await;
            secrets.push(challenge.clone());
            let response =
                factor_request(&harness.router, factor, &challenge, factor_code(factor)).await;
            assert_eq!(response.status.as_u16(), status);
            public(&response);
            if let Some(token) = response.json()["access_token"].as_str() {
                secrets.push(token.to_owned());
            }
            let request_id = response.headers["x-request-id"].to_str().unwrap();
            uuid::Uuid::parse_str(request_id).unwrap();
            assert!(!request_ids.iter().any(|id| id == request_id));
            request_ids.push(request_id.to_owned());
            let output = String::from_utf8(capture.0.lock().unwrap().clone()).unwrap();
            let events: Vec<Value> = output
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            assert_eq!(events.len(), request_ids.len());
            let fields = &events.last().unwrap()["fields"];
            assert_eq!(fields["request_id"], request_id);
            assert_eq!(fields["method"], factor);
            assert_eq!(fields["user_id"], id(1).to_string());
            assert_eq!(fields["result"], result);
            assert_eq!(fields["reason"], reason);
            assert_eq!(fields.as_object().unwrap().len(), 7);
            assert!(fields["timestamp_unix_ms"].as_i64().unwrap() > 0);
            for secret in &secrets {
                assert!(
                    !output.contains(secret),
                    "credential appeared in access log"
                );
            }
            for value in fields.as_object().unwrap().values() {
                assert_ne!(value.as_str(), Some("123456"));
            }
            assert!(!output.contains(EMAIL));
            assert!(fields.get("role").is_none());
            assert!(fields.get("user").is_none());
        }
    }
}
