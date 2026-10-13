mod provisioning;
#[path = "support.rs"]
mod support;

use application::identity::UserRepository;
use application::ApplicationError;
use domain::audit::AuditLog;
use infrastructure::{
    with_validated_ca_identity_postgres, with_validated_postgres, PostgresUserRepository,
};
use postgres::{error::SqlState, Client, NoTls};
use support::{search_path, Fixture};
use time::OffsetDateTime;

#[test]
fn existing_user_and_audit_adapters_open_only_the_minimal_ca_schema() {
    let mut db = Fixture::new();
    let (users, mut audit) = db.open().unwrap();
    let found = users.find_by_email(&db.operator.email).unwrap().unwrap();
    assert_eq!(found.id, db.operator.id);
    users
        .replace_recovery_codes(
            found.id,
            found.revision,
            found.recovery_codes.clone(),
            OffsetDateTime::now_utc(),
        )
        .unwrap();
    assert_eq!(users.find_by_id(found.id).unwrap().unwrap().revision, 1);
    audit
        .append(
            &found.email,
            "ca.test_observed",
            &found.id.to_string(),
            OffsetDateTime::now_utc(),
        )
        .unwrap();
    assert_eq!(audit.load_all().unwrap().len(), 2);
    db.assert_office_unchanged();
    let tables: Vec<String> = db.admin().query(
        "SELECT tablename::text FROM pg_catalog.pg_tables WHERE schemaname='qadra_ca' ORDER BY tablename", &[],
    ).unwrap().into_iter().map(|row| row.get(0)).collect();
    assert_eq!(tables, ["audit_events", "users"]);
}

#[test]
fn ca_opening_does_not_relax_global_validation_or_plain_url_sources() {
    let db = Fixture::new();
    assert!(PostgresUserRepository::open(&db.runtime).is_err());
    assert!(
        with_validated_postgres(&db.runtime, |_source| Ok::<_, ApplicationError>(()),).is_err()
    );
    db.open().unwrap();
}

#[test]
fn backup_reader_can_capture_ca_and_office_in_one_database_but_ca_runtime_cannot() {
    let mut db = Fixture::new();
    let mut backup = Client::connect(&db.backup, NoTls).unwrap();
    let current: String = backup
        .query_one("SELECT current_database()", &[])
        .unwrap()
        .get(0);
    assert_eq!(current, db.database);
    assert_eq!(
        backup
            .query_one("SELECT count(*) FROM qadra_ca.users", &[])
            .unwrap()
            .get::<_, i64>(0),
        1
    );
    assert_eq!(
        backup
            .query_one("SELECT count(*) FROM qadra_ca.audit_events", &[])
            .unwrap()
            .get::<_, i64>(0),
        0
    );
    assert_eq!(
        backup
            .query_one("SELECT marker FROM public.office_sentinel", &[])
            .unwrap()
            .get::<_, String>(0),
        "office-only"
    );
    let mut runtime = db.connect_runtime();
    let error = runtime
        .query("SELECT * FROM public.office_sentinel", &[])
        .unwrap_err();
    assert_eq!(error.code(), Some(&SqlState::INSUFFICIENT_PRIVILEGE));
    db.assert_office_unchanged();
}

#[test]
fn ca_runtime_has_no_ddl_enrollment_or_history_rewrite_authority() {
    let db = Fixture::new();
    let mut runtime = db.connect_runtime();
    for sql in [
        "CREATE TABLE qadra_ca.unexpected (value INTEGER)",
        "CREATE SCHEMA unexpected_ca_schema",
        "INSERT INTO qadra_ca.users SELECT * FROM qadra_ca.users WHERE false",
        "UPDATE qadra_ca.users SET password_hash=password_hash WHERE false",
        "UPDATE qadra_ca.users SET role=role WHERE false",
        "UPDATE qadra_ca.users SET active=active WHERE false",
        "UPDATE qadra_ca.users SET auth_generation=auth_generation WHERE false",
        "UPDATE qadra_ca.audit_events SET actor=actor WHERE false",
        "DELETE FROM qadra_ca.audit_events WHERE false",
        "TRUNCATE qadra_ca.audit_events",
    ] {
        let mut transaction = runtime.transaction().unwrap();
        let rejected = transaction.batch_execute(sql).unwrap_err();
        transaction.rollback().unwrap();
        assert_eq!(
            rejected.code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE),
            "{sql}"
        );
    }
}

#[test]
fn expected_database_and_role_are_checked_even_when_table_layout_matches() {
    let db = Fixture::new();
    for (database, role) in [
        ("not_the_configured_database", db.role.as_str()),
        (db.database.as_str(), db.backup_role.as_str()),
    ] {
        let result = with_validated_ca_identity_postgres(&db.runtime, database, role, |_source| {
            Ok::<_, ApplicationError>(())
        });
        assert!(result.is_err());
    }
}

#[test]
fn public_shadow_and_temporary_search_paths_are_rejected_without_fallback() {
    let db = Fixture::new();
    for path in [
        "public",
        "public,qadra_ca",
        "qadra_ca,public",
        "pg_temp,qadra_ca",
        "pg_catalog,qadra_ca",
    ] {
        let result = with_validated_ca_identity_postgres(
            &search_path(&db.runtime, path),
            &db.database,
            &db.role,
            |_source| Ok::<_, ApplicationError>(()),
        );
        assert!(result.is_err(), "unsafe path was admitted: {path}");
    }
}

#[test]
fn missing_required_or_excessive_effective_privileges_fail_startup() {
    let mut db = Fixture::new();
    let role = db.role.clone();
    for (change, restore) in [
        (
            format!("REVOKE INSERT ON qadra_ca.audit_events FROM {role}"),
            format!("GRANT INSERT ON qadra_ca.audit_events TO {role}"),
        ),
        (
            format!("GRANT UPDATE(actor) ON qadra_ca.audit_events TO {role}"),
            format!("REVOKE UPDATE(actor) ON qadra_ca.audit_events FROM {role}"),
        ),
        (
            format!("GRANT UPDATE(password_hash) ON qadra_ca.users TO {role}"),
            format!("REVOKE UPDATE(password_hash) ON qadra_ca.users FROM {role}"),
        ),
        (
            format!("GRANT CREATE ON SCHEMA qadra_ca TO {role}"),
            format!("REVOKE CREATE ON SCHEMA qadra_ca FROM {role}"),
        ),
        (
            format!("ALTER ROLE {role} BYPASSRLS"),
            format!("ALTER ROLE {role} NOBYPASSRLS"),
        ),
    ] {
        db.admin().batch_execute(&change).unwrap();
        db.assert_rejected();
        db.admin().batch_execute(&restore).unwrap();
        db.open().unwrap();
    }
}

#[test]
fn membership_that_can_assume_office_authority_is_not_a_ca_runtime() {
    let mut db = Fixture::new();
    let inherited = db.new_role();
    let role = db.role.clone();
    db.admin()
        .batch_execute(&format!(
            "GRANT USAGE ON SCHEMA public TO {inherited};
         GRANT SELECT ON public.office_sentinel TO {inherited};
         GRANT {inherited} TO {role}"
        ))
        .unwrap();
    db.assert_rejected();
}

#[test]
fn privilege_change_during_composition_is_rejected_even_with_the_same_target_names() {
    let mut db = Fixture::new();
    let runtime = db.runtime.clone();
    let database = db.database.clone();
    let role = db.role.clone();
    let result = with_validated_ca_identity_postgres(&runtime, &database, &role, |source| {
        db.admin()
            .batch_execute(&format!(
                "GRANT USAGE ON SCHEMA public TO {role};
                 GRANT SELECT ON public.office_sentinel TO {role}"
            ))
            .unwrap();
        PostgresUserRepository::open(source).map(|_| ())
    });
    assert!(
        result.is_err(),
        "post-validation privilege drift was admitted"
    );
}

#[test]
fn replacing_the_ca_schema_under_the_same_name_invalidates_the_startup_capability() {
    let mut db = Fixture::new();
    let runtime = db.runtime.clone();
    let database = db.database.clone();
    let role = db.role.clone();
    let result = with_validated_ca_identity_postgres(&runtime, &database, &role, |source| {
        db.admin()
            .batch_execute("DROP SCHEMA qadra_ca CASCADE")
            .unwrap();
        db.admin()
            .batch_execute(include_str!("../../src/ca_identity/schema.sql"))
            .unwrap();
        db.admin()
            .batch_execute(&format!(
                "GRANT USAGE ON SCHEMA qadra_ca TO {role};
                 GRANT SELECT ON qadra_ca.users, qadra_ca.audit_events TO {role};
                 GRANT INSERT ON qadra_ca.audit_events TO {role};
                 GRANT UPDATE(recovery_codes,revision,updated_at) ON qadra_ca.users TO {role}"
            ))
            .unwrap();
        PostgresUserRepository::open(source).map(|_| ())
    });
    assert!(
        result.is_err(),
        "same-name replacement schema reused a stale capability"
    );
}
