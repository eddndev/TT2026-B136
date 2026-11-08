//! An Owner may invite a Client account, the counterparty of a contract:
//! docs/adr/0086-bilateral-contract-signing.md.
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;

use application::{
    identity::invitations::*,
    identity::{Principal, UserRecord},
};
use case_administration_support::Fixture;
use domain::{
    clock::OffsetDateTime,
    crypto::{RecoveryCodeSet, Sha256Digest, RECOVERY_CODE_COUNT},
    identity::{Role, UserId},
};
use infrastructure::PostgresInvitationStore;

const EMAIL: &str = "client@example.test";
const NEW_CHECK: &str =
    "(role = ANY (ARRAY['litigator'::text, 'paralegal'::text, 'client'::text]))";

fn fixture() -> Fixture {
    Fixture::new().expect("client invitations require isolated CASE_TEST_DATABASE_URL")
}
fn actor(db: &Fixture) -> InvitationActor {
    InvitationActor {
        principal: Principal {
            id: db.owner,
            email: "owner@example.test".into(),
            role: Role::Owner,
        },
        revision: 0,
        auth_generation: 0,
    }
}
fn digest(n: u8) -> Sha256Digest {
    Sha256Digest::from_array([n; 32])
}
fn issue_client(db: &Fixture, store: &PostgresInvitationStore) -> InvitationSummary {
    store
        .issue(
            actor(db),
            IssueInvitation {
                request: InvitationIssueRequest {
                    operation_id: Uuid::new_v4(),
                    email: EMAIL.into(),
                    display_name: "Test Invitee".into(),
                    role: Role::Client,
                },
                token_digest: digest(1),
                protected_token: vec![3; 80],
                at: OffsetDateTime::now_utc(),
            },
        )
        .unwrap()
}
fn role_check(db: &mut Fixture) -> String {
    db.admin
        .query_one(
            "SELECT pg_get_expr(conbin,conrelid) FROM pg_constraint
            WHERE conrelid='member_invitations'::regclass AND conname='member_invitations_role_check'",
            &[],
        )
        .unwrap()
        .get(0)
}

#[test]
fn a_stored_client_invitation_keeps_startup_open_and_accepts_as_a_client_account() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let issued = issue_client(&db, &store);
    assert_eq!(issued.role, Role::Client);
    db.migrate();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    assert_eq!(store.get(actor(&db), issued.id).unwrap().role, Role::Client);
    let pending = PendingInvitation {
        operation_id: Uuid::new_v4(),
        request_digest: digest(3),
        protected_material: vec![7; 200],
    };
    let saved = store
        .save_pending(PrepareInvitation {
            id: issued.id,
            revision: 1,
            token_digest: digest(1),
            pending,
            at: OffsetDateTime::now_utc(),
        })
        .unwrap();
    let user = UserId::new();
    let receipt = store
        .finish(FinishInvitation {
            id: issued.id,
            revision: 1,
            pending_operation_id: saved.operation_id,
            operation_id: Uuid::new_v4(),
            request_digest: digest(4),
            token_digest: digest(1),
            user: UserRecord {
                id: user,
                email: EMAIL.into(),
                display_name: domain::identity::DisplayName::new("Test Invitee").unwrap(),
                role: Role::Client,
                password_hash: "fixture-password".into(),
                active: true,
                protected_totp_secret: vec![5; 80],
                recovery_codes: RecoveryCodeSet::from_hashes(
                    (0..RECOVERY_CODE_COUNT)
                        .map(|n| format!("recovery-{n}"))
                        .collect(),
                )
                .unwrap(),
                revision: 0,
                auth_generation: 0,
            },
            at: OffsetDateTime::now_utc(),
        })
        .unwrap();
    assert_eq!((receipt.user_id, receipt.role), (user, Role::Client));
    let role: String = db
        .admin
        .query_one("SELECT role FROM users WHERE id=$1", &[&user.as_uuid()])
        .unwrap()
        .get(0);
    assert_eq!(role, "client");
    db.migrate();
    PostgresInvitationStore::open(&db.runtime_url).unwrap();
}

#[test]
fn migration_widens_the_previous_role_constraint_and_startup_rejects_it_unwidened() {
    let mut db = fixture();
    assert_eq!(role_check(&mut db), NEW_CHECK);
    db.admin
        .batch_execute(
            "ALTER TABLE member_invitations DROP CONSTRAINT member_invitations_role_check;
            ALTER TABLE member_invitations ADD CONSTRAINT member_invitations_role_check
                CHECK(role IN ('litigator','paralegal'))",
        )
        .unwrap();
    assert!(PostgresInvitationStore::open(&db.runtime_url).is_err());
    db.migrate();
    assert_eq!(role_check(&mut db), NEW_CHECK);
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    issue_client(&db, &store);
    let error = db
        .admin
        .execute(
            "UPDATE member_invitations SET role='owner' WHERE email=$1",
            &[&EMAIL],
        )
        .unwrap_err();
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
}
