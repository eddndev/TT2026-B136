use std::sync::{Arc, Barrier};

use application::{identity::UserRepository, ApplicationError};
use domain::{
    audit::{verify_chain, AuditLog, ChainVerification},
    identity::{Role, UserId},
};
use infrastructure::{provision_initial_ca_operator, RingSha256Hasher};
use reqwest::Url;
use time::{Date, Month, OffsetDateTime};

use super::support::{search_path, Fixture};

fn empty_fixture() -> Fixture {
    let mut db = Fixture::new();
    db.admin()
        .batch_execute("DELETE FROM qadra_ca.users")
        .unwrap();
    db
}

fn admin_target(db: &mut Fixture) -> (String, String) {
    let role = db
        .admin()
        .query_one("SELECT current_user::text", &[])
        .unwrap()
        .get(0);
    let mut url = Url::parse(&std::env::var("IDENTITY_TEST_DATABASE_URL").unwrap()).unwrap();
    url.set_path(&format!("/{}", db.database));
    (search_path(url.as_str(), "qadra_ca,pg_temp"), role)
}

fn assert_counts(db: &mut Fixture, users: i64, events: i64) {
    let row = db
        .admin()
        .query_one(
            "SELECT (SELECT count(*) FROM qadra_ca.users),
                (SELECT count(*) FROM qadra_ca.audit_events)",
            &[],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), users);
    assert_eq!(row.get::<_, i64>(1), events);
    db.assert_office_unchanged();
}

#[test]
fn initial_operator_and_ca_audit_commit_together_without_changing_office_or_runtime() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    let user = db.operator.clone();
    let at = OffsetDateTime::from_unix_timestamp(1_735_689_600).unwrap();
    provision_initial_ca_operator(&url, &db.database, &owner, user.clone(), at).unwrap();

    let (users, audit) = db.open().unwrap();
    let stored = users.find_by_id(user.id).unwrap().unwrap();
    assert_eq!(stored.email, user.email);
    assert_eq!(stored.role, Role::Client);
    assert!(stored.active);
    assert!(stored.password_hash == user.password_hash);
    assert!(stored.protected_totp_secret == user.protected_totp_secret);
    assert!(stored.recovery_codes == user.recovery_codes);
    assert_eq!(stored.revision, 0);
    assert_eq!(stored.auth_generation, 0);
    let events = audit.load_all().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event.actor, owner);
    assert_eq!(events[0].event.action, "ca.operator_enrolled");
    assert_eq!(events[0].event.resource, user.id.to_string());
    assert_eq!(events[0].event.timestamp, at);
    assert_eq!(
        verify_chain(&RingSha256Hasher, &events).unwrap(),
        ChainVerification::Valid { entries: 1 }
    );
    let repeated = provision_initial_ca_operator(&url, &db.database, &owner, user, at);
    assert!(matches!(repeated, Err(ApplicationError::UserAlreadyExists)));
    assert_counts(&mut db, 1, 1);
}

#[test]
fn any_existing_account_closes_initial_enrollment_even_when_inactive_and_different() {
    let mut db = Fixture::new();
    db.admin()
        .batch_execute("UPDATE qadra_ca.users SET active=false")
        .unwrap();
    let (url, owner) = admin_target(&mut db);
    let mut other = db.operator.clone();
    other.id = UserId::new();
    other.email = "another-operator@example.test".into();
    let result =
        provision_initial_ca_operator(&url, &db.database, &owner, other, OffsetDateTime::now_utc());
    assert!(matches!(result, Err(ApplicationError::UserAlreadyExists)));
    assert_counts(&mut db, 1, 0);
    assert!(!db
        .admin()
        .query_one("SELECT active FROM qadra_ca.users", &[])
        .unwrap()
        .get::<_, bool>(0));
}

#[test]
fn provisioning_rejects_another_database_or_role_and_the_runtime_or_backup_identity() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    for (database, role) in [
        ("different_database", owner.as_str()),
        (db.database.as_str(), db.role.as_str()),
    ] {
        let result = provision_initial_ca_operator(
            &url,
            database,
            role,
            db.operator.clone(),
            OffsetDateTime::now_utc(),
        );
        assert!(matches!(
            result,
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
    for (url, role) in [(&db.runtime, &db.role), (&db.backup, &db.backup_role)] {
        let result = provision_initial_ca_operator(
            url,
            &db.database,
            role,
            db.operator.clone(),
            OffsetDateTime::now_utc(),
        );
        assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
    }
    assert_counts(&mut db, 0, 0);
}

#[test]
fn provisioning_does_not_fall_back_to_public_or_accept_a_reordered_search_path() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    for path in [
        "public",
        "public,qadra_ca",
        "qadra_ca,public",
        "pg_temp,qadra_ca",
    ] {
        let result = provision_initial_ca_operator(
            &search_path(&url, path),
            &db.database,
            &owner,
            db.operator.clone(),
            OffsetDateTime::now_utc(),
        );
        assert!(matches!(
            result,
            Err(ApplicationError::InvalidConfiguration(_))
        ));
    }
    assert_counts(&mut db, 0, 0);
}

#[test]
fn provisioning_validates_the_exact_ca_layout_and_table_owner_before_writing() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    db.admin()
        .batch_execute("ALTER TABLE qadra_ca.users ADD COLUMN unexpected TEXT")
        .unwrap();
    let result = provision_initial_ca_operator(
        &url,
        &db.database,
        &owner,
        db.operator.clone(),
        OffsetDateTime::now_utc(),
    );
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert_counts(&mut db, 0, 0);
    db.admin()
        .batch_execute("ALTER TABLE qadra_ca.users DROP COLUMN unexpected")
        .unwrap();
    let runtime_role = db.role.clone();
    db.admin()
        .batch_execute(&format!(
            "ALTER TABLE qadra_ca.users OWNER TO {runtime_role}"
        ))
        .unwrap();
    let result = provision_initial_ca_operator(
        &url,
        &db.database,
        &owner,
        db.operator.clone(),
        OffsetDateTime::now_utc(),
    );
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert_counts(&mut db, 0, 0);
}

#[test]
fn provisioning_does_not_create_a_missing_ca_schema() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    db.admin()
        .batch_execute("DROP SCHEMA qadra_ca CASCADE")
        .unwrap();
    assert!(provision_initial_ca_operator(
        &url,
        &db.database,
        &owner,
        db.operator.clone(),
        OffsetDateTime::now_utc()
    )
    .is_err());
    assert!(!db
        .admin()
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM pg_catalog.pg_namespace WHERE nspname='qadra_ca')",
            &[],
        )
        .unwrap()
        .get::<_, bool>(0));
    db.assert_office_unchanged();
}

#[test]
fn provisioning_only_accepts_a_new_active_client_record() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    let valid = db.operator.clone();
    let mut owner_record = valid.clone();
    owner_record.role = Role::Owner;
    let mut inactive = valid.clone();
    inactive.active = false;
    let mut revised = valid.clone();
    revised.revision = 1;
    let mut rotated = valid.clone();
    rotated.auth_generation = 1;
    let mut nil = valid;
    nil.id = UserId::from_uuid(uuid::Uuid::nil());
    for user in [owner_record, inactive, revised, rotated, nil] {
        assert!(provision_initial_ca_operator(
            &url,
            &db.database,
            &owner,
            user,
            OffsetDateTime::now_utc()
        )
        .is_err());
        assert_counts(&mut db, 0, 0);
    }
}

#[test]
fn audit_encoding_failure_rolls_back_the_new_ca_account() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    // The audit encoder rejects a negative year in its RFC 3339 timestamp.
    let at = Date::from_calendar_date(-1, Month::January, 1)
        .unwrap()
        .midnight()
        .assume_utc();
    assert!(
        provision_initial_ca_operator(&url, &db.database, &owner, db.operator.clone(), at).is_err()
    );
    assert_counts(&mut db, 0, 0);
}

#[test]
fn competing_initial_enrollments_commit_only_one_account_and_one_audit_event() {
    let mut db = empty_fixture();
    let (url, owner) = admin_target(&mut db);
    let mut other = db.operator.clone();
    other.id = UserId::new();
    other.email = "competing-operator@example.test".into();
    let start = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = [db.operator.clone(), other]
            .into_iter()
            .map(|user| {
                let start = start.clone();
                let (url, database, owner) = (&url, &db.database, &owner);
                scope.spawn(move || {
                    start.wait();
                    provision_initial_ca_operator(
                        url,
                        database,
                        owner,
                        user,
                        OffsetDateTime::now_utc(),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(ApplicationError::UserAlreadyExists)))
            .count(),
        1
    );
    assert_counts(&mut db, 1, 1);
}
