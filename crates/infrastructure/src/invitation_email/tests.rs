use application::identity::invitations::{
    InvitationDelivery, InvitationDeliveryLease, InvitationId,
};
use application::identity::password_reset::ResetDeliveryOutcome;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use std::time::{Duration, Instant};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::invitation_email::{InvitationEmailConfiguration, ResendInvitationDelivery};

#[path = "../password_reset_email/test_support.rs"]
mod support;
use support::{envelope, Response, Server, KEY, RECEIPT, TOKEN};

fn configuration() -> InvitationEmailConfiguration {
    InvitationEmailConfiguration::new(
        "invitations@example.test".into(),
        "https://qadra.example.test/".into(),
    )
    .unwrap()
}

fn sender() -> ResendInvitationDelivery {
    ResendInvitationDelivery::new(
        Zeroizing::new(KEY.into()),
        configuration(),
        Duration::from_millis(100),
        Duration::from_secs(1),
    )
    .unwrap()
}

fn local(server: &Server) -> ResendInvitationDelivery {
    sender()
        .with_test_endpoint(server.endpoint.clone())
        .unwrap()
}

fn lease() -> InvitationDeliveryLease {
    let envelope = envelope();
    InvitationDeliveryLease {
        id: InvitationId::from_uuid(Uuid::from_u128(1)),
        revision: 2,
        delivery_id: Uuid::from_u128(3),
        generation: 4,
        lease_id: Uuid::from_u128(5),
        email: envelope.email,
        protected_token: b"ciphertext-not-for-email".to_vec(),
        expires_at: envelope.expires_at,
        lease_until: envelope.expires_at,
        provider_deadline: envelope.expires_at,
    }
}

fn outcome(
    sender: &ResendInvitationDelivery,
    lease: &InvitationDeliveryLease,
) -> ResetDeliveryOutcome {
    sender
        .deliver(lease, Zeroizing::new(TOKEN.to_vec()))
        .unwrap()
}

#[test]
fn invitation_email_has_a_fixed_endpoint_and_validated_explicit_configuration() {
    assert_eq!(
        sender().endpoint_for_test(),
        "https://api.resend.com/emails"
    );
    for (from, url) in [
        ("private-marker", "https://qadra.example.test/"),
        (
            "x@example.test\r\nBcc:private-marker",
            "https://qadra.example.test/",
        ),
        ("x@example.test", "http://qadra.example.test/"),
        (
            "x@example.test",
            "https://private-marker@qadra.example.test/",
        ),
        (
            "x@example.test",
            "https://qadra.example.test/private-marker",
        ),
        (
            "x@example.test",
            "https://qadra.example.test/?private-marker",
        ),
        (
            "x@example.test",
            "https://qadra.example.test/#private-marker",
        ),
    ] {
        let error = match InvitationEmailConfiguration::new(from.into(), url.into()) {
            Ok(_) => panic!("unsafe invitation configuration accepted"),
            Err(error) => error,
        };
        assert!(!format!("{error:?} {error}").contains("private-marker"));
    }
    for (connect, total) in [(0, 1), (1, 0), (2, 1), (4, 10), (1, 11)] {
        assert!(ResendInvitationDelivery::new(
            Zeroizing::new(KEY.into()),
            configuration(),
            Duration::from_secs(connect),
            Duration::from_secs(total),
        )
        .is_err());
    }
    for key in [
        String::new(),
        "private-key-marker\n".into(),
        "a".repeat(1025),
    ] {
        let error = match ResendInvitationDelivery::new(
            Zeroizing::new(key),
            configuration(),
            Duration::from_secs(1),
            Duration::from_secs(2),
        ) {
            Ok(_) => panic!("invalid invitation API key accepted"),
            Err(error) => error,
        };
        assert!(!format!("{error:?} {error}").contains("private-key-marker"));
    }
}

#[test]
fn invitation_email_retries_keep_the_exact_body_and_delivery_id() {
    let server = Server::new(Response::reply(200, RECEIPT));
    let sender = local(&server);
    let original = lease();
    assert_eq!(outcome(&sender, &original), ResetDeliveryOutcome::Accepted);
    let mut retry = original.clone();
    retry.lease_id = Uuid::from_u128(6);
    retry.lease_until -= time::Duration::seconds(1);
    assert_eq!(outcome(&sender, &retry), ResetDeliveryOutcome::Accepted);
    let requests = server.finish();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    for request in requests {
        assert_eq!(request.method, "POST");
        assert_eq!(request.target, "/emails");
        assert_eq!(
            request.headers["idempotency-key"],
            format!("invitation:{}", original.delivery_id)
        );
        assert_eq!(request.headers["authorization"], format!("Bearer {KEY}"));
        let encoded = URL_SAFE_NO_PAD.encode(TOKEN);
        for (name, value) in request.headers {
            if name != "authorization" {
                assert!(!value.contains(KEY) && !value.contains(&encoded));
            }
        }
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(body.as_object().unwrap().len(), 4);
        assert_eq!(body["from"], "invitations@example.test");
        assert_eq!(body["to"], serde_json::json!(["recipient@example.test"]));
        assert_eq!(body["subject"], "Qadra: invitacion para crear tu cuenta");
        let text = body["text"].as_str().unwrap();
        let links: Vec<_> = text
            .split_whitespace()
            .filter(|s| s.starts_with("https://"))
            .collect();
        assert_eq!(links.len(), 1);
        let url = reqwest::Url::parse(links[0]).unwrap();
        assert_eq!(
            url.origin().ascii_serialization(),
            "https://qadra.example.test"
        );
        assert_eq!(url.path(), "/");
        assert!(url.query().is_none() && url.username().is_empty() && url.password().is_none());
        assert_eq!(
            url.fragment(),
            Some(format!("invitation={encoded}").as_str())
        );
        assert!(text.contains("2030-01-01T00:00:00Z"));
        assert!(!text.contains(KEY) && !text.contains("recipient@example.test"));
        assert!(!text.contains("ciphertext-not-for-email"));
    }
}

#[test]
fn invitation_email_classifies_provider_replies_without_automatic_retry() {
    for (status, body, expected) in [
        (201, RECEIPT, ResetDeliveryOutcome::Accepted),
        (
            400,
            "{\"name\":\"validation_error\"}",
            ResetDeliveryOutcome::DefinitelyRejected,
        ),
        (
            429,
            "{\"name\":\"rate_limit_exceeded\"}",
            ResetDeliveryOutcome::DefinitelyRejected,
        ),
        (
            409,
            "{\"name\":\"invalid_idempotent_request\"}",
            ResetDeliveryOutcome::Uncertain,
        ),
        (
            500,
            "{\"name\":\"private-provider-marker\"}",
            ResetDeliveryOutcome::Uncertain,
        ),
        (200, "{}", ResetDeliveryOutcome::Uncertain),
        (
            200,
            "[\"0e6d1d44-17c3-4d1b-b32e-d8ffcf91681e\",null]",
            ResetDeliveryOutcome::Uncertain,
        ),
    ] {
        let server = Server::new(Response::reply(status, body));
        assert_eq!(outcome(&local(&server), &lease()), expected);
        assert_eq!(server.finish().len(), 1);
    }
    let server = Server::new(Response::Disconnect);
    assert_eq!(
        outcome(&local(&server), &lease()),
        ResetDeliveryOutcome::Uncertain
    );
    assert_eq!(server.finish().len(), 1);
    let server = Server::new(Response::reply(200, "x".repeat(8193)));
    assert_eq!(
        outcome(&local(&server), &lease()),
        ResetDeliveryOutcome::Uncertain
    );
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn invitation_email_never_follows_redirects_or_unbounded_replies() {
    let target = Server::new(Response::reply(200, RECEIPT));
    let source = Server::new(Response::Reply {
        status: 307,
        body: "{}".into(),
        declared: None,
        headers: vec![("Location".into(), target.endpoint.clone())],
    });
    assert_eq!(
        outcome(&local(&source), &lease()),
        ResetDeliveryOutcome::Uncertain
    );
    assert_eq!(source.finish().len(), 1);
    assert_eq!(target.finish().len(), 0);
    let server = Server::new(Response::Stall);
    let sender = ResendInvitationDelivery::new(
        Zeroizing::new(KEY.into()),
        configuration(),
        Duration::from_millis(100),
        Duration::from_millis(200),
    )
    .unwrap()
    .with_test_endpoint(server.endpoint.clone())
    .unwrap();
    let start = Instant::now();
    assert_eq!(outcome(&sender, &lease()), ResetDeliveryOutcome::Uncertain);
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn invitation_email_rejects_invalid_recipient_token_and_delivery_id_before_io() {
    let server = Server::new(Response::reply(200, RECEIPT));
    let sender = local(&server);
    let mut invalid = lease();
    invalid.email = "private-marker\r\nBcc:other@example.test".into();
    assert_eq!(
        outcome(&sender, &invalid),
        ResetDeliveryOutcome::DefinitelyRejected
    );
    for size in [0, 31, 33] {
        assert_eq!(
            sender
                .deliver(&lease(), Zeroizing::new(vec![0; size]))
                .unwrap(),
            ResetDeliveryOutcome::DefinitelyRejected
        );
    }
    let mut invalid = lease();
    invalid.delivery_id = Uuid::nil();
    assert_eq!(
        outcome(&sender, &invalid),
        ResetDeliveryOutcome::DefinitelyRejected
    );
    assert_eq!(server.finish().len(), 0);
}
