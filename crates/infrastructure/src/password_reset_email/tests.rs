use crate::password_reset_email::{PasswordResetEmailConfiguration, ResendPasswordResetDelivery};
use application::identity::password_reset::{PasswordResetDelivery, ResetDeliveryOutcome};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

#[path = "response_tests.rs"]
mod response_tests;
#[path = "test_support.rs"]
mod support;
use support::{envelope, Response, Server, KEY, RECEIPT, TOKEN};

fn configuration() -> PasswordResetEmailConfiguration {
    PasswordResetEmailConfiguration::new(
        "recovery@example.test".into(),
        "https://qadra.example.test/".into(),
    )
    .unwrap_or_else(|_| panic!("valid email configuration was rejected"))
}

fn sender() -> ResendPasswordResetDelivery {
    ResendPasswordResetDelivery::new(
        Zeroizing::new(KEY.into()),
        configuration(),
        Duration::from_millis(100),
        Duration::from_secs(1),
    )
    .unwrap_or_else(|_| panic!("valid email adapter configuration was rejected"))
}

fn local(server: &Server) -> ResendPasswordResetDelivery {
    sender()
        .with_test_endpoint(server.endpoint.clone())
        .unwrap_or_else(|_| panic!("loopback fixture endpoint was rejected"))
}

fn outcome(sender: &ResendPasswordResetDelivery) -> ResetDeliveryOutcome {
    sender
        .deliver(envelope())
        .unwrap_or_else(|_| panic!("delivery returned an unexpected error"))
}

#[test]
fn production_endpoint_is_fixed_and_no_constructor_network_call_is_needed() {
    assert_eq!(
        sender().endpoint_for_test(),
        "https://api.resend.com/emails"
    );
}

#[test]
fn sender_and_fixed_https_root_are_validated_without_echoing_private_values() {
    for (from, url) in [
        ("", "https://qadra.example.test/"),
        ("private-address-marker", "https://qadra.example.test/"),
        (
            "recovery@example.test\r\nBcc: private-marker",
            "https://qadra.example.test/",
        ),
        ("recovery@example.test", "http://qadra.example.test/"),
        ("recovery@example.test", "http://127.0.0.1/"),
        (
            "recovery@example.test",
            "https://private-marker@qadra.example.test/",
        ),
        (
            "recovery@example.test",
            "https://qadra.example.test/?token=private-marker",
        ),
        (
            "recovery@example.test",
            "https://qadra.example.test/#private-marker",
        ),
        (
            "recovery@example.test",
            "https://qadra.example.test/cases/private-marker",
        ),
        ("recovery@example.test", "https://qadra.example.test/\n"),
    ] {
        let error = match PasswordResetEmailConfiguration::new(from.into(), url.into()) {
            Ok(_) => panic!("unsafe email configuration was accepted"),
            Err(error) => error,
        };
        assert!(
            !error.to_string().contains("private-marker"),
            "configuration error leaked input"
        );
    }
    let long = format!("{}@example.test", "a".repeat(321));
    assert!(
        PasswordResetEmailConfiguration::new(long, "https://qadra.example.test/".into()).is_err()
    );
}

#[test]
fn keys_and_timeouts_are_explicit_bounded_and_secret_free_on_error() {
    for key in [
        String::new(),
        "private-key-marker\n".into(),
        "x".repeat(1025),
    ] {
        let error = match ResendPasswordResetDelivery::new(
            Zeroizing::new(key),
            configuration(),
            Duration::from_secs(1),
            Duration::from_secs(2),
        ) {
            Ok(_) => panic!("invalid key was accepted"),
            Err(error) => error,
        };
        assert!(
            !error.to_string().contains("private-key-marker"),
            "configuration error leaked key"
        );
    }
    for (connect, total) in [
        (Duration::ZERO, Duration::from_secs(1)),
        (Duration::from_secs(1), Duration::ZERO),
        (Duration::from_secs(2), Duration::from_secs(1)),
        (Duration::from_secs(4), Duration::from_secs(10)),
        (Duration::from_secs(1), Duration::from_secs(11)),
        (Duration::MAX, Duration::MAX),
    ] {
        assert!(ResendPasswordResetDelivery::new(
            Zeroizing::new(KEY.into()),
            configuration(),
            connect,
            total
        )
        .is_err());
    }
}

#[test]
fn accepted_message_has_one_canonical_fragment_link_and_no_secret_metadata() {
    for status in [200, 201] {
        let server = Server::new(Response::reply(status, RECEIPT));
        assert_eq!(outcome(&local(&server)), ResetDeliveryOutcome::Accepted);
        let requests = server.finish();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.method, "POST");
        assert_eq!(request.target, "/emails");
        assert!(
            request
                .headers
                .get("authorization")
                .is_some_and(|value| value == &format!("Bearer {KEY}")),
            "API credential was not confined to the authorization header"
        );
        let encoded = URL_SAFE_NO_PAD.encode(TOKEN);
        for (name, value) in &request.headers {
            if name != "authorization" {
                assert!(
                    !value.contains(KEY) && !value.contains(&encoded),
                    "private material leaked to a header"
                );
            }
        }
        let key = request
            .headers
            .get("idempotency-key")
            .expect("missing independent submission ID");
        let id = uuid::Uuid::parse_str(key).expect("submission ID is not a UUID");
        assert!(
            !id.is_nil() && id.to_string() == *key,
            "submission ID is not canonical"
        );
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        let object = body.as_object().expect("email body is not an object");
        assert_eq!(object.len(), 4);
        assert!(
            body["from"] == "recovery@example.test"
                && body["to"] == serde_json::json!(["recipient@example.test"]),
            "configured sender or recipient changed"
        );
        assert!(
            body["subject"] == "Qadra: restablece tu contrasena",
            "unexpected subject"
        );
        let text = body["text"].as_str().expect("missing plain email text");
        let links: Vec<_> = text
            .split_whitespace()
            .filter(|word| word.starts_with("https://"))
            .collect();
        assert_eq!(links.len(), 1);
        let link = reqwest::Url::parse(links[0]).unwrap();
        assert!(
            link.origin().ascii_serialization() == "https://qadra.example.test"
                && link.path() == "/",
            "configured reset destination changed"
        );
        assert!(
            link.query().is_none() && link.username().is_empty() && link.password().is_none(),
            "reset link contains query or credentials"
        );
        let fragment = link
            .fragment()
            .and_then(|value| value.strip_prefix("password-reset="))
            .expect("missing dedicated reset fragment");
        assert!(
            fragment.len() == 43 && fragment == encoded,
            "token encoding changed"
        );
        assert!(
            URL_SAFE_NO_PAD
                .decode(fragment)
                .is_ok_and(|value| value == TOKEN),
            "token bytes changed"
        );
        assert!(
            text.contains("2030-01-01T00:00:00Z"),
            "original expiry was omitted"
        );
        assert!(
            !text.contains(KEY) && !text.contains("recipient@example.test"),
            "unneeded private data in text"
        );
    }
}

#[test]
fn distinct_deliveries_never_use_the_capability_as_the_submission_id() {
    let mut ids = Vec::new();
    for _ in 0..2 {
        let server = Server::new(Response::reply(200, RECEIPT));
        assert_eq!(outcome(&local(&server)), ResetDeliveryOutcome::Accepted);
        let request = server.finish().pop().unwrap();
        ids.push(request.headers["idempotency-key"].clone());
    }
    assert!(
        ids[0] != ids[1],
        "independent deliveries reused a submission ID"
    );
}

#[test]
fn recognized_provider_rejections_are_definite_and_never_retried() {
    for (status, name) in [
        (400, "validation_error"),
        (401, "missing_api_key"),
        (401, "restricted_api_key"),
        (403, "restricted_api_key"),
        (403, "suspended_api_key"),
        (403, "invalid_permission"),
        (403, "validation_error"),
        (422, "missing_required_field"),
        (422, "invalid_parameter"),
        (429, "rate_limit_exceeded"),
        (429, "daily_quota_exceeded"),
        (429, "monthly_quota_exceeded"),
    ] {
        let body =
            serde_json::json!({"name":name,"message":"private-provider-message"}).to_string();
        let server = Server::new(Response::reply(status, body));
        assert_eq!(
            outcome(&local(&server)),
            ResetDeliveryOutcome::DefinitelyRejected
        );
        assert_eq!(server.finish().len(), 1);
    }
}

#[test]
fn conflicts_unknown_errors_and_server_failures_preserve_uncertainty() {
    for (status, body) in [
        (409, "{\"name\":\"concurrent_idempotent_requests\"}"),
        (409, "{\"name\":\"invalid_idempotent_request\"}"),
        (408, "{}"),
        (429, "{}"),
        (401, "not-json"),
        (422, "{\"name\":\"new_error\"}"),
        (500, "{\"name\":\"application_error\"}"),
        (503, "{\"name\":\"service_unavailable\"}"),
        (299, "{}"),
    ] {
        let server = Server::new(Response::reply(status, body));
        assert_eq!(outcome(&local(&server)), ResetDeliveryOutcome::Uncertain);
        assert_eq!(server.finish().len(), 1);
    }
}

#[test]
fn malformed_or_incomplete_success_never_confirms_acceptance() {
    for body in [
        "not-json",
        "{}",
        "{\"id\":\"private-provider-marker\"}",
        "{\"id\":42}",
    ] {
        let server = Server::new(Response::reply(200, body));
        assert_eq!(outcome(&local(&server)), ResetDeliveryOutcome::Uncertain);
        assert_eq!(server.finish().len(), 1);
    }
    let server = Server::new(Response::Reply {
        status: 200,
        body: RECEIPT.into(),
        headers: Vec::new(),
        declared: Some(RECEIPT.len() + 20),
    });
    assert_eq!(outcome(&local(&server)), ResetDeliveryOutcome::Uncertain);
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn response_body_limit_is_enforced_at_exactly_eight_kibibytes() {
    for (size, expected) in [
        (8192, ResetDeliveryOutcome::Accepted),
        (8193, ResetDeliveryOutcome::Uncertain),
    ] {
        let body = format!("{RECEIPT}{}", " ".repeat(size - RECEIPT.len()));
        let server = Server::new(Response::reply(200, body));
        assert_eq!(outcome(&local(&server)), expected);
        assert_eq!(server.finish().len(), 1);
    }
}

#[test]
fn redirects_do_not_forward_credentials_or_capabilities() {
    let target = Server::new(Response::reply(200, RECEIPT));
    let source = Server::new(Response::Reply {
        status: 307,
        body: "{}".into(),
        headers: vec![("Location".into(), target.endpoint.clone())],
        declared: None,
    });
    assert_eq!(outcome(&local(&source)), ResetDeliveryOutcome::Uncertain);
    assert_eq!(source.finish().len(), 1);
    assert_eq!(target.finish().len(), 0);
}

#[test]
fn lost_reply_is_uncertain_and_does_not_repeat_submission() {
    let server = Server::new(Response::Disconnect);
    assert_eq!(outcome(&local(&server)), ResetDeliveryOutcome::Uncertain);
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn configured_total_timeout_bounds_a_stalled_reply_without_retry() {
    let server = Server::new(Response::Stall);
    let sender = ResendPasswordResetDelivery::new(
        Zeroizing::new(KEY.into()),
        configuration(),
        Duration::from_millis(100),
        Duration::from_millis(200),
    )
    .unwrap()
    .with_test_endpoint(server.endpoint.clone())
    .unwrap();
    let started = Instant::now();
    assert_eq!(outcome(&sender), ResetDeliveryOutcome::Uncertain);
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "delivery ignored its total timeout"
    );
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn invalid_recipient_is_rejected_before_any_network_submission() {
    for recipient in [
        String::new(),
        "not-an-address".into(),
        "private-marker@example.test\r\nBcc: other@example.test".into(),
        format!("{}@example.test", "a".repeat(321)),
    ] {
        let server = Server::new(Response::reply(200, RECEIPT));
        let mut value = envelope();
        value.email = recipient;
        let result = local(&server).deliver(value);
        assert!(
            matches!(result, Ok(ResetDeliveryOutcome::DefinitelyRejected)),
            "invalid recipient was not rejected locally"
        );
        assert_eq!(server.finish().len(), 0);
    }
}
