use crate::password_reset_http_support::*;
use axum::http::StatusCode;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

#[tokio::test]
async fn canonical_completion_decodes_exact_bytes_and_returns_empty_success_without_a_session() {
    let fixture = Harness::ready();
    assert_eq!(token().len(), 43);
    let reply = post(&fixture.router, COMPLETE, completion(&token(), PASSWORD)).await;
    reply.public();
    assert_eq!(reply.status, StatusCode::NO_CONTENT);
    assert!(reply.body.is_empty());
    assert!(!reply.headers.contains_key("content-type"));
    assert_eq!(
        fixture.ports.calls(),
        ["digest", "limit", "inspect", "hash", "complete"]
    );
    assert_eq!(*fixture.requests.calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn malformed_noncanonical_and_unusable_capabilities_share_one_rejection() {
    let mut trailing_bits = token();
    assert!(trailing_bits.ends_with('o'));
    trailing_bits.pop();
    trailing_bits.push('p');
    let mut baseline = None;
    for malformed in [
        String::new(),
        format!("{}=", token()),
        format!(" {}", token()),
        URL_SAFE_NO_PAD.encode([0x5a; 31]),
        URL_SAFE_NO_PAD.encode([0x5a; 33]),
        "5a".repeat(32),
        "+".repeat(43),
        "/".repeat(43),
        "\u{e9}".repeat(43),
        trailing_bits,
    ] {
        let fixture = Harness::ready();
        let reply = post(&fixture.router, COMPLETE, completion(&malformed, PASSWORD)).await;
        reply.error(StatusCode::BAD_REQUEST, "password_reset_rejected");
        fixture.untouched();
        if let Some(expected) = &baseline {
            assert!(&reply == expected);
        } else {
            baseline = Some(reply);
        }
    }
    for mode in [Mode::Limited, Mode::Missing, Mode::Rejected] {
        let fixture = Harness::ready();
        *fixture.ports.mode.lock().unwrap() = mode;
        let reply = post(&fixture.router, COMPLETE, completion(&token(), PASSWORD)).await;
        reply.error(StatusCode::BAD_REQUEST, "password_reset_rejected");
        assert!(
            Some(reply) == baseline,
            "rejection exposed capability state"
        );
        match mode {
            Mode::Limited => assert_eq!(fixture.ports.count("inspect"), 0),
            Mode::Missing => assert_eq!(fixture.ports.count("hash"), 0),
            Mode::Rejected => assert_eq!(fixture.ports.count("complete"), 1),
            _ => unreachable!(),
        }
    }
}

#[tokio::test]
async fn password_policy_counts_bytes_before_lookup_and_returns_only_a_safe_known_error() {
    let mut baseline = None;
    for password in [
        "x".repeat(11),
        "x".repeat(1025),
        format!("{}x", "\u{e9}".repeat(512)),
    ] {
        let fixture = Harness::ready();
        let reply = post(&fixture.router, COMPLETE, completion(&token(), &password)).await;
        reply.error(StatusCode::BAD_REQUEST, "invalid_password");
        fixture.untouched();
        if let Some(expected) = &baseline {
            assert!(&reply == expected);
        } else {
            baseline = Some(reply);
        }
    }
    for password in ["x".repeat(12), "x".repeat(1024), "\u{e9}".repeat(6)] {
        let fixture = Harness::ready();
        let reply = post(&fixture.router, COMPLETE, completion(&token(), &password)).await;
        reply.public();
        assert_eq!(reply.status, StatusCode::NO_CONTENT);
        assert_eq!(fixture.ports.count("complete"), 1);
    }
    let fixture = Harness::ready();
    post(&fixture.router, COMPLETE, completion("!", "short"))
        .await
        .error(StatusCode::BAD_REQUEST, "password_reset_rejected");
    fixture.untouched();
}

#[tokio::test]
async fn storage_uncertainty_private_invalid_input_and_hash_failure_never_leak_or_retry() {
    let mut baseline = None;
    for mode in [Mode::Uncertain, Mode::InvalidInput, Mode::HashError] {
        let fixture = Harness::ready();
        *fixture.ports.mode.lock().unwrap() = mode;
        let reply = post(&fixture.router, COMPLETE, completion(&token(), PASSWORD)).await;
        reply.error(StatusCode::SERVICE_UNAVAILABLE, "password_reset_uncertain");
        assert_eq!(fixture.ports.count("inspect"), 1);
        assert!(fixture.ports.count("hash") <= 1);
        assert!(fixture.ports.count("complete") <= 1);
        if let Some(expected) = &baseline {
            assert!(&reply == expected);
        } else {
            baseline = Some(reply);
        }
    }
}
