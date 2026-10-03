use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::identity::UserId;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn start_returns_exact_public_statement_without_session_or_client_authority() {
    let (router, workflow) = standalone();
    let reply = post(&router, START, start_body()).await;
    assert_eq!(reply.status, StatusCode::OK);
    public(&reply);
    assert_eq!(
        reply.json(),
        json!({"challenge_token": token(),
        "statement_base64": STANDARD.encode(statement().canonical_bytes()),
        "expires_in_seconds": 299})
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Start(
            UserId::from_uuid(Uuid::parse_str(OWNER).unwrap()),
            Uuid::parse_str(BINDING).unwrap()
        )]
    );
}

#[tokio::test]
async fn proof_delegates_exact_bytes_and_returns_only_mandatory_mfa_challenge() {
    let (router, workflow) = standalone();
    let request = Request::post(PROOF)
        .header("content-type", "application/json")
        .header("authorization", "Bearer unrelated-stale-session")
        .body(Body::from(proof_body()))
        .unwrap();
    let reply = send(&router, request).await;
    assert_eq!(reply.status, StatusCode::OK);
    public(&reply);
    assert_eq!(
        reply.json(),
        json!({"challenge_token": mfa_token(), "expires_in_seconds": 300})
    );
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Proof(token(), signature())]
    );
}

#[tokio::test]
async fn application_rejection_is_not_retried_or_replaced_by_another_start() {
    let (router, workflow) = standalone();
    *workflow.outcome.lock().unwrap() = Outcome::Invalid;
    let reply = post(&router, PROOF, proof_body()).await;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    assert_eq!(reply.json()["error"]["code"], "invalid_credentials");
    public(&reply);
    assert_eq!(
        *workflow.calls.lock().unwrap(),
        vec![Call::Proof(token(), signature())]
    );
}

#[tokio::test]
async fn authority_quota_and_port_failures_keep_generic_statuses_and_private_details_out() {
    for (outcome, status, code) in [
        (
            Outcome::Invalid,
            StatusCode::UNAUTHORIZED,
            "invalid_credentials",
        ),
        (
            Outcome::Locked,
            StatusCode::TOO_MANY_REQUESTS,
            "account_locked",
        ),
        (
            Outcome::Port,
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
        ),
    ] {
        for (path, body) in [(START, start_body()), (PROOF, proof_body())] {
            let (router, workflow) = standalone();
            *workflow.outcome.lock().unwrap() = outcome;
            let reply = post(&router, path, body).await;
            assert_eq!(reply.status, status);
            assert_eq!(reply.json()["error"]["code"], code);
            public(&reply);
            let text = String::from_utf8(reply.body).unwrap();
            for private in [
                token(),
                OWNER.to_owned(),
                BINDING.to_owned(),
                "private-owner-login-port".to_owned(),
            ] {
                assert!(
                    !text.contains(&private),
                    "error response disclosed private input"
                );
            }
            assert_eq!(workflow.calls.lock().unwrap().len(), 1);
        }
    }
}
