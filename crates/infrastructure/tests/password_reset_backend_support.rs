#![allow(dead_code)]

pub use crate::case_administration_support::Fixture;
pub use application::identity::password_reset::{
    CompleteReset, IssueReset, PasswordResetRepository, ResetCandidate, ResetCompletion, ResetId,
    ResetIssue, ResetIssueOutcome, ResetPolicy,
};
use domain::audit::{AuditLog, ChainVerification};
pub use domain::crypto::Sha256Digest;
use domain::crypto::{RecoveryCodeSet, RECOVERY_CODE_COUNT};
pub use domain::identity::UserId;
pub use infrastructure::identity::PostgresPasswordResetRepository;
use infrastructure::{PostgresAuditLog, RingSha256Hasher};

pub fn fixture() -> Fixture {
    Fixture::new().expect("password reset backend tests require CASE_TEST_DATABASE_URL")
}

pub fn store(db: &Fixture) -> PostgresPasswordResetRepository {
    PostgresPasswordResetRepository::open(&db.runtime_url).unwrap()
}

pub fn digest(value: u8) -> Sha256Digest {
    Sha256Digest::from_array([value; 32])
}

pub fn account(db: &mut Fixture, role: &str, active: bool) -> UserId {
    let id = UserId::new();
    let recovery = RecoveryCodeSet::from_hashes(
        (0..RECOVERY_CODE_COUNT)
            .map(|slot| format!("fixture-code-{slot}"))
            .collect(),
    )
    .unwrap();
    let recovery = serde_json::to_value(recovery).unwrap();
    db.admin.execute(
        "INSERT INTO users(id,email,password_hash,role,active,protected_totp_secret,recovery_codes)
         VALUES($1,$2,'original-hash',$3,$4,$5,$6)",
        &[&id.as_uuid(), &email(id), &role, &active, &vec![3u8; 48], &recovery],
    ).unwrap();
    db.admin
        .execute(
            "INSERT INTO case_memberships(case_id,user_id) VALUES($1,$2)",
            &[&db.case.as_uuid(), &id.as_uuid()],
        )
        .unwrap();
    id
}

pub fn email(id: UserId) -> String {
    format!("{id}@example.test")
}

pub fn command(id: UserId, value: u8, capacity: u32) -> IssueReset {
    IssueReset {
        email: email(id),
        digest: digest(value),
        policy: ResetPolicy::new(60, capacity).unwrap(),
    }
}

pub fn issue(repository: &PostgresPasswordResetRepository, id: UserId, value: u8) -> ResetIssue {
    match repository.issue(command(id, value, 2)).unwrap() {
        ResetIssueOutcome::Issued(issued) => issued,
        ResetIssueOutcome::Ignored => panic!("active account with capacity must issue"),
    }
}

pub fn complete(candidate: ResetCandidate, value: u8, hash: &str) -> CompleteReset {
    CompleteReset {
        digest: digest(value),
        candidate,
        password_hash: hash.into(),
    }
}

pub fn snapshot(db: &mut Fixture) -> serde_json::Value {
    db.admin.query_one(
        "SELECT jsonb_build_object(
         'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
         'members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id,user_id) FROM case_memberships m),
         'capabilities',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM password_reset_capabilities r),
         'audit',(SELECT jsonb_agg(to_jsonb(a) ORDER BY sequence) FROM audit_events a))", &[],
    ).unwrap().get(0)
}

pub fn stable_identity(db: &mut Fixture, id: UserId) -> serde_json::Value {
    db.admin.query_one(
        "SELECT jsonb_build_object('user',to_jsonb(u)-ARRAY['password_hash','revision','auth_generation','updated_at'],
         'members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY case_id) FROM case_memberships m WHERE user_id=u.id))
         FROM users u WHERE id=$1", &[&id.as_uuid()],
    ).unwrap().get(0)
}

pub fn password_state(db: &mut Fixture, id: UserId) -> (String, i64, i64) {
    let row = db
        .admin
        .query_one(
            "SELECT password_hash,revision,auth_generation FROM users WHERE id=$1",
            &[&id.as_uuid()],
        )
        .unwrap();
    (row.get(0), row.get(1), row.get(2))
}

pub fn reset_events(db: &mut Fixture) -> i64 {
    db.admin
        .query_one(
            "SELECT count(*) FROM audit_events WHERE action='identity.password_reset'",
            &[],
        )
        .unwrap()
        .get(0)
}

pub fn assert_chain(db: &Fixture) {
    let events = PostgresAuditLog::open(&db.runtime_url)
        .unwrap()
        .load_all()
        .unwrap();
    assert_eq!(
        domain::audit::verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid {
            entries: events.len()
        },
    );
}

pub fn expire(db: &mut Fixture, id: ResetId) {
    db.admin.execute(
        "UPDATE password_reset_capabilities SET issued_at=clock_timestamp()-interval '2 minutes',
         expires_at=clock_timestamp()-interval '1 minute' WHERE id=$1", &[&id.as_uuid()],
    ).unwrap();
}

pub fn raw_consume_sql(db: &Fixture) -> String {
    format!(
        "SELECT (extract(epoch FROM reset_at)*1000000)::bigint AS reset_at_micros,
         next_audit,new_revision,new_generation FROM {}.password_reset_consume($1,$2,$3,$4,$5)",
        db.schema,
    )
}
