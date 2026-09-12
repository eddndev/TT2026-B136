use domain::clock::OffsetDateTime;
use std::env;

use application::identity::{UserRecord, UserRepository};
use domain::crypto::{RecoveryCodeSet, RECOVERY_CODE_COUNT};
use domain::identity::{Role, UserId};
use infrastructure::PostgresUserRepository;
use postgres::{Client, NoTls};
use uuid::Uuid;

fn user(email: &str, role: Role) -> UserRecord {
    UserRecord {
        id: UserId::new(),
        email: email.into(),
        password_hash: "test-hash".into(),
        role,
        active: true,
        protected_totp_secret: vec![1; 48],
        recovery_codes: RecoveryCodeSet::from_hashes(vec!["test-code".into(); RECOVERY_CODE_COUNT])
            .unwrap(),
        revision: 0,
    }
}

#[test]
fn an_audit_failure_rolls_back_bootstrap_and_preserves_retry() {
    let Ok(url) = env::var("CASE_TEST_DATABASE_URL") else {
        return;
    };
    let schema = format!("identity_tx_{}", Uuid::new_v4().simple());
    let mut control = Client::connect(&url, NoTls).unwrap();
    control
        .batch_execute(&format!(
            "CREATE SCHEMA {schema}; SET search_path TO {schema}"
        ))
        .unwrap();
    let separator = if url.contains('?') { '&' } else { '?' };
    let scoped = format!("{url}{separator}options=-csearch_path%3D{schema}");
    let repository = PostgresUserRepository::connect(&scoped).unwrap();
    control
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS audit_events (
        sequence BIGINT PRIMARY KEY, timestamp TEXT NOT NULL, actor TEXT NOT NULL,
        action TEXT NOT NULL, resource TEXT NOT NULL, chain BYTEA NOT NULL);
        CREATE FUNCTION reject_audit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'injected audit failure'; END $$;
        CREATE TRIGGER reject_audit BEFORE INSERT ON audit_events
        FOR EACH ROW EXECUTE FUNCTION reject_audit();",
        )
        .unwrap();
    let owner = user("owner@example.com", Role::Owner);
    let result = repository.insert_initial_owner(owner.clone(), OffsetDateTime::now_utc());
    let remaining: i64 = control
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    control
        .batch_execute("DROP TRIGGER reject_audit ON audit_events")
        .unwrap();
    let retry = repository.insert_initial_owner(owner, OffsetDateTime::now_utc());
    control
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .unwrap();
    assert!(
        result.is_err(),
        "bootstrap must fail when its audit event cannot commit"
    );
    assert_eq!(
        remaining, 0,
        "failed bootstrap must leave enrolment available"
    );
    assert!(retry.unwrap());
}

#[test]
fn user_creation_and_recovery_updates_roll_back_when_audit_commit_fails() {
    let Ok(url) = env::var("CASE_TEST_DATABASE_URL") else {
        return;
    };
    let schema = format!("identity_commit_{}", Uuid::new_v4().simple());
    let mut control = Client::connect(&url, NoTls).unwrap();
    control
        .batch_execute(&format!(
            "CREATE SCHEMA {schema}; SET search_path TO {schema}"
        ))
        .unwrap();
    let separator = if url.contains('?') { '&' } else { '?' };
    let scoped = format!("{url}{separator}options=-csearch_path%3D{schema}");
    let repository = PostgresUserRepository::connect(&scoped).unwrap();
    let owner = user("owner@example.com", Role::Owner);
    repository
        .insert_initial_owner(owner.clone(), OffsetDateTime::now_utc())
        .unwrap();
    let assistant = user("assistant@example.com", Role::Paralegal);
    control
        .batch_execute(
            "CREATE FUNCTION reject_commit() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN RAISE EXCEPTION 'injected commit failure'; END $$;
        CREATE CONSTRAINT TRIGGER reject_commit AFTER INSERT ON audit_events
        DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION reject_commit();",
        )
        .unwrap();
    let created = repository.insert(assistant.clone(), owner.id, OffsetDateTime::now_utc());
    let codes = RecoveryCodeSet::from_hashes(vec!["changed".into(); RECOVERY_CODE_COUNT]).unwrap();
    let consumed = repository.replace_recovery_codes(owner.id, 0, codes, OffsetDateTime::now_utc());
    let stored = repository.find_by_id(owner.id).unwrap().unwrap();
    let absent = repository.find_by_id(assistant.id).unwrap().is_none();
    let audit_count: i64 = control
        .query_one("SELECT COUNT(*) FROM audit_events", &[])
        .unwrap()
        .get(0);
    control
        .batch_execute("DROP TRIGGER reject_commit ON audit_events")
        .unwrap();
    repository
        .insert(assistant, owner.id, OffsetDateTime::now_utc())
        .unwrap();
    control
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .unwrap();
    assert!(created.is_err());
    assert!(consumed.is_err());
    assert!(absent);
    assert_eq!(stored.revision, 0);
    assert_eq!(stored.recovery_codes, owner.recovery_codes);
    assert_eq!(audit_count, 1);
}

#[test]
fn persistence_rechecks_current_owner_before_creating_a_user() {
    let Ok(url) = env::var("CASE_TEST_DATABASE_URL") else {
        return;
    };
    let schema = format!("identity_role_{}", Uuid::new_v4().simple());
    let mut control = Client::connect(&url, NoTls).unwrap();
    control
        .batch_execute(&format!(
            "CREATE SCHEMA {schema}; SET search_path TO {schema}"
        ))
        .unwrap();
    let separator = if url.contains('?') { '&' } else { '?' };
    let scoped = format!("{url}{separator}options=-csearch_path%3D{schema}");
    let repository = PostgresUserRepository::connect(&scoped).unwrap();
    let owner = user("owner@example.com", Role::Owner);
    repository
        .insert_initial_owner(owner.clone(), OffsetDateTime::now_utc())
        .unwrap();
    control
        .execute(
            "UPDATE users SET role='paralegal' WHERE id=$1",
            &[&owner.id.as_uuid()],
        )
        .unwrap();
    let denied = repository.insert(
        user("new@example.com", Role::Client),
        owner.id,
        OffsetDateTime::now_utc(),
    );
    let count: i64 = control
        .query_one("SELECT COUNT(*) FROM users", &[])
        .unwrap()
        .get(0);
    control
        .batch_execute(&format!("DROP SCHEMA {schema} CASCADE"))
        .unwrap();
    assert!(matches!(
        denied,
        Err(application::ApplicationError::PermissionDenied)
    ));
    assert_eq!(count, 1);
}
