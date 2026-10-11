use super::*;
use crate::password_reset_email::test_support::{Response, Server, KEY, RECEIPT};
use std::time::Instant;

fn configuration() -> CaseReportEmailConfiguration {
    CaseReportEmailConfiguration::new(
        "reports@example.test".into(),
        "https://qadra.example.test/".into(),
    )
    .unwrap()
}

fn sender() -> ResendCaseReportEmailDelivery {
    ResendCaseReportEmailDelivery::new(
        Zeroizing::new(KEY.into()),
        Duration::from_millis(100),
        Duration::from_secs(1),
    )
    .unwrap()
}

fn local(server: &Server) -> ResendCaseReportEmailDelivery {
    sender()
        .with_test_endpoint(server.endpoint.clone())
        .unwrap()
}

fn message() -> CaseReportEmailMessage {
    let template = configuration().template();
    CaseReportEmailMessage {
        from: template.from,
        to: "requester@example.test".into(),
        subject: template.subject,
        text: template.text,
    }
}

fn delivery_id() -> Uuid {
    Uuid::from_u128(35)
}

fn outcome(sender: &ResendCaseReportEmailDelivery) -> CaseReportEmailOutcome {
    sender.deliver(delivery_id(), &message()).unwrap()
}

#[test]
fn configured_template_is_generic_and_links_only_to_authenticated_reports() {
    let first = configuration().template();
    assert_eq!(first, configuration().template());
    assert_eq!(first.from, "reports@example.test");
    assert_eq!(first.subject, "Qadra: informe disponible");
    let links: Vec<_> = first
        .text
        .split_whitespace()
        .filter(|word| word.starts_with("https://"))
        .collect();
    assert_eq!(links, ["https://qadra.example.test/#reports"]);
    assert!(!first.text.contains("requester@example.test"));
    assert!(!first.text.contains(KEY));
    assert!(!first.text.contains(&delivery_id().to_string()));
    assert!(!first.text.contains("token=") && !first.text.contains("password="));
    assert_eq!(
        sender().endpoint_for_test(),
        "https://api.resend.com/emails"
    );
}

#[test]
fn configuration_rejects_unsafe_origins_and_sender_without_echoing_input() {
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
        (
            "x@example.test",
            "https://qadra.example.test/\\private-marker",
        ),
    ] {
        let error = match CaseReportEmailConfiguration::new(from.into(), url.into()) {
            Ok(_) => panic!("unsafe report email configuration accepted"),
            Err(error) => error,
        };
        assert!(!format!("{error:?} {error}").contains("private-marker"));
    }
}

#[test]
fn transport_rejects_invalid_secret_and_timeouts_without_secret_diagnostics() {
    for (connect, total) in [(0, 1), (1, 0), (2, 1), (4, 10), (1, 11)] {
        assert!(ResendCaseReportEmailDelivery::new(
            Zeroizing::new(KEY.into()),
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
        let error = match ResendCaseReportEmailDelivery::new(
            Zeroizing::new(key),
            Duration::from_secs(1),
            Duration::from_secs(2),
        ) {
            Ok(_) => panic!("invalid report email API key accepted"),
            Err(error) => error,
        };
        assert!(!format!("{error:?} {error}").contains("private-key-marker"));
    }
}

#[test]
fn retries_send_exact_persisted_envelope_and_stable_provider_key_without_attachments() {
    let server = Server::new(Response::reply(200, RECEIPT));
    let original = CaseReportEmailMessage {
        from: "original@example.test".into(),
        to: "original-recipient@example.test".into(),
        subject: "Original \"persisted\" subject".into(),
        text: "Original text\n\nhttps://original.example.test/#reports".into(),
    };
    for _ in 0..2 {
        assert_eq!(
            local(&server).deliver(delivery_id(), &original).unwrap(),
            CaseReportEmailOutcome::Accepted
        );
    }
    let requests = server.finish();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body, requests[1].body);
    for request in requests {
        assert_eq!(request.method, "POST");
        assert_eq!(request.target, "/emails");
        assert_eq!(
            request.headers["idempotency-key"],
            format!("case-report-ready:{}", delivery_id())
        );
        assert_eq!(request.headers["authorization"], format!("Bearer {KEY}"));
        assert_eq!(request.headers["content-type"], "application/json");
        let body: serde_json::Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "from": original.from, "to": [original.to],
                "subject": original.subject, "text": original.text,
            })
        );
        assert!(!String::from_utf8(request.body).unwrap().contains(KEY));
        for (name, value) in request.headers {
            if name != "authorization" {
                assert!(!value.contains(KEY));
            }
        }
    }
}

#[test]
fn provider_replies_map_to_finite_outcomes_without_automatic_retry() {
    for (status, body, expected) in [
        (201, RECEIPT, CaseReportEmailOutcome::Accepted),
        (
            400,
            "{\"name\":\"validation_error\"}",
            CaseReportEmailOutcome::DefinitelyRejected,
        ),
        (
            429,
            "{\"name\":\"rate_limit_exceeded\"}",
            CaseReportEmailOutcome::DefinitelyRejected,
        ),
        (
            409,
            "{\"name\":\"invalid_idempotent_request\"}",
            CaseReportEmailOutcome::Uncertain,
        ),
        (
            500,
            "{\"name\":\"private-provider-marker\"}",
            CaseReportEmailOutcome::Uncertain,
        ),
        (200, "{}", CaseReportEmailOutcome::Uncertain),
        (
            200,
            "[\"0e6d1d44-17c3-4d1b-b32e-d8ffcf91681e\",null]",
            CaseReportEmailOutcome::Uncertain,
        ),
    ] {
        let server = Server::new(Response::reply(status, body));
        let result = outcome(&local(&server));
        assert_eq!(result, expected);
        assert!(!format!("{result:?}").contains("private-provider-marker"));
        assert_eq!(server.finish().len(), 1);
    }
    let server = Server::new(Response::Disconnect);
    assert_eq!(outcome(&local(&server)), CaseReportEmailOutcome::Uncertain);
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn redirects_and_oversized_responses_are_uncertain_without_following_or_retry() {
    let target = Server::new(Response::reply(200, RECEIPT));
    let source = Server::new(Response::Reply {
        status: 307,
        body: "{}".into(),
        declared: None,
        headers: vec![("Location".into(), target.endpoint.clone())],
    });
    assert_eq!(outcome(&local(&source)), CaseReportEmailOutcome::Uncertain);
    assert_eq!(source.finish().len(), 1);
    assert_eq!(target.finish().len(), 0);
    let server = Server::new(Response::reply(200, "x".repeat(8193)));
    assert_eq!(outcome(&local(&server)), CaseReportEmailOutcome::Uncertain);
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn stalled_provider_is_bounded_by_total_timeout() {
    let server = Server::new(Response::Stall);
    let sender = ResendCaseReportEmailDelivery::new(
        Zeroizing::new(KEY.into()),
        Duration::from_millis(100),
        Duration::from_millis(200),
    )
    .unwrap()
    .with_test_endpoint(server.endpoint.clone())
    .unwrap();
    let started = Instant::now();
    assert_eq!(outcome(&sender), CaseReportEmailOutcome::Uncertain);
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(server.finish().len(), 1);
}

#[test]
fn invalid_envelope_or_nil_delivery_is_rejected_before_io() {
    let server = Server::new(Response::reply(200, RECEIPT));
    let sender = local(&server);
    for field in [
        "from",
        "to",
        "subject",
        "text",
        "long_from",
        "long_to",
        "long_subject",
        "long_text",
    ] {
        let mut invalid = message();
        match field {
            "from" => invalid.from = "private-marker\r\nBcc:other@example.test".into(),
            "to" => invalid.to = "private-marker\r\nBcc:other@example.test".into(),
            "subject" => invalid.subject = "private-marker\r\nchanged".into(),
            "long_from" => invalid.from = format!("{}@example.test", "x".repeat(242)),
            "long_to" => invalid.to = format!("{}@example.test", "x".repeat(242)),
            "long_subject" => invalid.subject = "x".repeat(201),
            "long_text" => invalid.text = "x".repeat(4097),
            _ => invalid.text.clear(),
        }
        assert_eq!(
            sender.deliver(delivery_id(), &invalid).unwrap(),
            CaseReportEmailOutcome::DefinitelyRejected
        );
    }
    assert_eq!(
        sender.deliver(Uuid::nil(), &message()).unwrap(),
        CaseReportEmailOutcome::DefinitelyRejected
    );
    assert_eq!(server.finish().len(), 0);
}
