use crate::serve_invitation_acceptance_support::*;
use crate::serve_password_reset_http_acceptance_support::me;
use application::identity::invitations::{InvitationDeliveryRun, InvitationStore};
use axum::http::StatusCode;
use domain::{
    audit::{AuditLog, ChainVerification},
    identity::UserId,
};
use infrastructure::{PostgresAuditLog, PostgresInvitationStore, RingSha256Hasher};
use serde_json::json;
use uuid::Uuid;

#[test]
fn real_invitation_acceptance_survives_reentry_and_resend_without_duplicate_accounts() {
    let mut flow = Acceptance::new();
    let owner = flow.owner();
    let email = format!("invited-{}@example.test", Uuid::new_v4().simple());
    let issue_operation = Uuid::new_v4();
    let issued = flow.post(
        OWNER_PATH,
        Some(&owner),
        json!({
            "operation_id":issue_operation,"email":email,"role":"litigator",
        }),
    );
    assert_eq!(issued.status, StatusCode::CREATED);
    issued.metadata();
    assert_eq!(issued.json()["revision"], "1");
    let delivery = flow.deliver();
    assert_eq!(delivery.id.as_uuid(), issue_operation);
    assert_eq!(delivery.email, email);
    assert_eq!(delivery.revision, 1);
    let operation = Uuid::new_v4();
    let input = recipient(&delivery.token, operation);
    let prepared = flow.post(&format!("{PUBLIC_PATH}/prepare"), None, input.clone());
    assert_eq!(prepared.status, StatusCode::OK);
    prepared.no_session();
    assert_eq!(
        flow.user_count(&email),
        0,
        "preparation created a usable account"
    );
    assert_eq!(flow.accepted_count(), 0);
    let secret = prepared.secret("totp_secret_base32");
    let accepted_code = code(&secret);
    let mut confirmation = input.clone();
    confirmation["code"] = json!(accepted_code.as_str());
    let accepted = flow.post(&format!("{PUBLIC_PATH}/finish"), None, confirmation.clone());
    assert_eq!(accepted.status, StatusCode::OK);
    accepted.metadata();
    let receipt = accepted.json();
    assert_eq!(receipt["email"], email);
    assert_eq!(receipt["operation_id"], operation.to_string());
    assert_eq!(flow.user_count(&email), 1);
    assert_eq!(flow.accepted_count(), 1);
    let replay = flow.post(&format!("{PUBLIC_PATH}/finish"), None, confirmation);
    assert_eq!(replay.status, StatusCode::OK);
    replay.metadata();
    assert_eq!(replay.json(), receipt);
    let consulted = flow.post(&format!("{PUBLIC_PATH}/receipt"), None, input.clone());
    assert_eq!(consulted.status, StatusCode::OK);
    consulted.metadata();
    assert_eq!(consulted.json(), json!({"receipt":receipt}));

    let reopened = PostgresInvitationStore::open(&flow.db.runtime_url).unwrap();
    let stored = reopened
        .completed(digest(&delivery.token), operation)
        .unwrap()
        .unwrap();
    assert_eq!(stored.receipt.operation_id, operation);
    assert_eq!(stored.receipt.email, email);
    drop(reopened);
    flow.reopen();
    let consulted = flow.post(&format!("{PUBLIC_PATH}/receipt"), None, input);
    assert_eq!(consulted.status, StatusCode::OK);
    consulted.metadata();
    assert_eq!(consulted.json(), json!({"receipt":receipt}));
    assert_eq!(flow.user_count(&email), 1);
    assert_eq!(flow.accepted_count(), 1);
    assert_eq!((flow.worker)().unwrap(), InvitationDeliveryRun::Idle);

    let user_id = UserId::from_uuid(receipt["user_id"].as_str().unwrap().parse().unwrap());
    let session = flow.authenticate(&email, user_id, &secret);
    assert_eq!(
        flow.runtime.block_on(me(&flow.router, &session)).status,
        StatusCode::OK
    );

    let second_email = format!("resent-{}@example.test", Uuid::new_v4().simple());
    let second_id = Uuid::new_v4();
    assert_eq!(
        flow.post(
            OWNER_PATH,
            Some(&owner),
            json!({
                "operation_id":second_id,"email":second_email,"role":"paralegal",
            })
        )
        .status,
        StatusCode::CREATED
    );
    let old = flow.deliver();
    let resent = flow.post(
        &format!("{OWNER_PATH}/{second_id}/resend"),
        Some(&owner),
        json!({
            "operation_id":Uuid::new_v4(),"expected_revision":"1",
        }),
    );
    assert_eq!(resent.status, StatusCode::OK);
    resent.metadata();
    assert_eq!(resent.json()["revision"], "2");
    let current = flow.deliver();
    assert_eq!(current.id, old.id);
    assert_eq!(current.revision, 2);
    assert!(
        current.token.as_slice() != old.token.as_slice(),
        "resend retained the old capability"
    );
    let rejected = flow.post(
        &format!("{PUBLIC_PATH}/prepare"),
        None,
        recipient(&old.token, Uuid::new_v4()),
    );
    assert_eq!(rejected.status, StatusCode::BAD_REQUEST);
    rejected.metadata();
    assert_eq!(rejected.json()["error"]["code"], "invitation_rejected");
    let prepared = flow.post(
        &format!("{PUBLIC_PATH}/prepare"),
        None,
        recipient(&current.token, Uuid::new_v4()),
    );
    assert_eq!(prepared.status, StatusCode::OK);
    prepared.no_session();
    assert_eq!(flow.user_count(&second_email), 0);
    assert_eq!(flow.user_count(&email), 1);
    assert_eq!(flow.accepted_count(), 1);
    let users: i64 = flow
        .db
        .admin
        .query_one("SELECT count(*) FROM users", &[])
        .unwrap()
        .get(0);
    assert_eq!(users, 2, "reentry or resend created another account");

    let audit = PostgresAuditLog::open(&flow.db.runtime_url).unwrap();
    let events = audit.load_all().unwrap();
    assert!(
        matches!(domain::audit::verify_chain(&RingSha256Hasher, &events),
        Ok(ChainVerification::Valid { entries }) if entries == events.len())
    );
    for (action, count) in [
        ("identity.invitation_issued", 2),
        ("identity.invitation_prepared", 2),
        ("identity.invitation_accepted", 1),
        ("identity.invitation_resent", 1),
    ] {
        assert_eq!(
            events
                .iter()
                .filter(|entry| entry.event.action == action)
                .count(),
            count
        );
    }
}
