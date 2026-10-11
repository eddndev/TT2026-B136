#[path = "case_administration_support/mod.rs"]
mod case_administration_support;

use application::{
    identity::invitations::{
        InvitationActor, InvitationDeliveryClaim, InvitationId, InvitationIssueRequest,
        InvitationStore, IssueInvitation, Uuid,
    },
    identity::Principal,
    ApplicationError,
};
use case_administration_support::Fixture;
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::Role};
use infrastructure::PostgresInvitationStore;
use postgres::error::SqlState;

fn fixture() -> Fixture {
    Fixture::new().expect("invitation boundaries require isolated CASE_TEST_DATABASE_URL")
}

fn issued(db: &Fixture, store: &PostgresInvitationStore) -> InvitationId {
    store
        .issue(
            InvitationActor {
                principal: Principal {
                    id: db.owner,
                    email: "owner@example.test".into(),
                    role: Role::Owner,
                },
                revision: 0,
                auth_generation: 0,
            },
            IssueInvitation {
                request: InvitationIssueRequest {
                    operation_id: Uuid::new_v4(),
                    email: "invited@example.test".into(),
                    role: Role::Litigator,
                },
                token_digest: Sha256Digest::from_array([1; 32]),
                protected_token: vec![3; 80],
                at: OffsetDateTime::now_utc(),
            },
        )
        .unwrap()
        .id
}

fn assert_null_state_rejected(assignment: &str) {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let id = issued(&db, &store);
    let mut tx = db.admin.transaction().unwrap();
    let result = tx.execute(
        &format!("UPDATE member_invitations SET {assignment} WHERE id=$1"),
        &[&id.as_uuid()],
    );
    tx.rollback().unwrap();
    assert_eq!(
        result.as_ref().err().and_then(|error| error.code()),
        Some(&SqlState::CHECK_VIOLATION),
        "incomplete invitation state must fail its SQL check: {assignment}"
    );
}

#[test]
fn pending_invitation_requires_encrypted_delivery_token() {
    assert_null_state_rejected("protected_token=NULL");
}

#[test]
fn pending_mfa_requires_a_request_digest() {
    assert_null_state_rejected(
        "pending_operation='10000000-0000-4000-8000-000000000001', \
         pending_digest=NULL, pending_material=decode(repeat('01',200),'hex')",
    );
}

#[test]
fn pending_mfa_requires_encrypted_material() {
    assert_null_state_rejected(
        "pending_operation='10000000-0000-4000-8000-000000000001', \
         pending_digest=decode(repeat('01',32),'hex'), pending_material=NULL",
    );
}

fn assert_runtime_rejects_immutable_update(column: &str) {
    let mut db = fixture();
    PostgresInvitationStore::open(&db.runtime_url).unwrap();
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE({column}) ON member_invitations TO {}",
            db.role
        ))
        .unwrap();
    let rejected = matches!(
        PostgresInvitationStore::open(&db.runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    );
    db.admin
        .batch_execute(&format!(
            "REVOKE UPDATE({column}) ON member_invitations FROM {}",
            db.role
        ))
        .unwrap();
    PostgresInvitationStore::open(&db.runtime_url).unwrap();
    assert!(
        rejected,
        "runtime must reject UPDATE permission on invitation {column}"
    );
}

#[test]
fn runtime_rejects_update_permission_on_invitation_email() {
    assert_runtime_rejects_immutable_update("email");
}

#[test]
fn runtime_rejects_update_permission_on_invitation_role() {
    assert_runtime_rejects_immutable_update("role");
}

#[test]
fn runtime_rejects_update_permission_on_invitation_id() {
    assert_runtime_rejects_immutable_update("id");
}

#[test]
fn delivery_lease_never_outlives_provider_idempotency_window() {
    let db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let id = issued(&db, &store);
    let lease = store
        .claim_delivery(InvitationDeliveryClaim {
            lease_id: Uuid::new_v4(),
            at: OffsetDateTime::now_utc(),
            lease_seconds: 30,
            provider_window_seconds: 1,
        })
        .unwrap()
        .unwrap();
    assert_eq!(lease.id, id);
    assert!(
        lease.lease_until <= lease.provider_deadline,
        "delivery lease must expire within the provider idempotency window"
    );
}
