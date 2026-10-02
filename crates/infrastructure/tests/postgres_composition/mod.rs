use crate::case_administration_support::Fixture;
use application::{identity::UserRepository, ApplicationError};
use infrastructure::{with_validated_postgres, PostgresAuditLog, PostgresUserRepository};
use std::{cell::Cell, panic::AssertUnwindSafe};

mod reconnect;
mod support;
use support::*;

#[test]
fn composed_adapters_have_separate_connections_without_repeating_inventory() {
    let Some(mut db) = Fixture::new() else { return };
    let before = db.snapshot();
    let (url, application) = bounded_url(&db);
    let users = with_validated_postgres(&url, |source| {
        assert!(!mutation_lock_available(&mut db.admin));
        let users = PostgresUserRepository::open(source)?;
        let audit = PostgresAuditLog::open(source)?;
        let pids = backend_pids(&mut db.admin, &application);
        assert_eq!(pids.len(), 3, "validation plus two independent adapters");
        assert!(
            PostgresUserRepository::open(&url).is_err(),
            "ordinary opening must still validate and respect mutation exclusion"
        );
        Ok::<_, ApplicationError>((users, audit))
    })
    .unwrap();
    assert!(mutation_lock_available(&mut db.admin));
    assert!(users.0.has_users().unwrap());
    assert_eq!(db.snapshot(), before);
    drop(users);
}

#[test]
fn scoped_validation_releases_mutation_exclusion_on_callback_error_and_panic() {
    let Some(mut db) = Fixture::new() else { return };
    let (url, _) = bounded_url(&db);
    let result = with_validated_postgres(&url, |_| {
        assert!(!mutation_lock_available(&mut db.admin));
        Err::<(), _>(ApplicationError::InvalidInput(
            "construction rejected".into(),
        ))
    });
    assert!(matches!(result, Err(ApplicationError::InvalidInput(_))));
    assert!(mutation_lock_available(&mut db.admin));
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _: Result<(), ApplicationError> = with_validated_postgres(&url, |_| {
            assert!(!mutation_lock_available(&mut db.admin));
            panic!("construction interrupted")
        });
    }));
    assert!(result.is_err());
    assert!(mutation_lock_available(&mut db.admin));
}

#[test]
fn scoped_validation_denies_an_administrative_role_before_composition() {
    let Some(mut db) = Fixture::new() else { return };
    let entered = Cell::new(false);
    let result = with_validated_postgres(&db.admin_url, |_| {
        entered.set(true);
        Ok::<_, ApplicationError>(())
    });
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidConfiguration(_))
    ));
    assert!(!entered.get());
    assert!(mutation_lock_available(&mut db.admin));
}

#[test]
fn scoped_validation_denies_unrelated_corrupt_inventory_without_repairing_it() {
    let Some(mut db) = Fixture::new() else { return };
    corrupt_case_actor_email(&mut db);
    let before = db.snapshot();
    let entered = Cell::new(false);
    let result = with_validated_postgres(&db.runtime_url, |_| {
        entered.set(true);
        Ok::<_, ApplicationError>(())
    });
    assert_inventory_error(result.err().unwrap());
    assert!(!entered.get());
    assert_eq!(db.snapshot(), before);
}

#[test]
fn ordinary_connections_recheck_inventory_after_a_successful_scope() {
    let Some(mut db) = Fixture::new() else { return };
    with_validated_postgres(&db.runtime_url, |_| Ok::<_, ApplicationError>(())).unwrap();
    corrupt_case_actor_email(&mut db);
    assert_inventory_error(
        PostgresUserRepository::open(db.runtime_url.as_str())
            .err()
            .unwrap(),
    );
    assert_inventory_error(PostgresUserRepository::open(&db.runtime_url).err().unwrap());
    let protected = zeroize::Zeroizing::new(db.runtime_url.clone());
    assert_inventory_error(PostgresUserRepository::open(&protected).err().unwrap());
}

#[test]
fn scoped_connections_reject_replaced_role_identity_at_the_same_url() {
    let Some(mut db) = Fixture::new() else { return };
    let renamed = format!("replaced_{}", uuid::Uuid::new_v4().simple());
    let replaced = Cell::new(false);
    let (url, _) = bounded_url(&db);
    let result = with_validated_postgres(&url, |source| {
        db.admin
            .batch_execute(&format!(
                "ALTER ROLE {} RENAME TO {renamed};
             CREATE ROLE {} LOGIN NOSUPERUSER NOCREATEROLE PASSWORD 'runtime-test-only'",
                db.role, db.role,
            ))
            .unwrap();
        replaced.set(true);
        PostgresUserRepository::open(source).map(|_| ())
    });
    if replaced.get() {
        db.admin
            .batch_execute(&format!(
                "DROP ROLE {}; ALTER ROLE {renamed} RENAME TO {}",
                db.role, db.role,
            ))
            .unwrap();
    }
    assert!(
        replaced.get(),
        "composition must reach the role replacement"
    );
    let error = result.expect_err("replaced role must not inherit the capability");
    assert!(
        matches!(error, ApplicationError::InvalidConfiguration(ref message)
        if message == "startup PostgreSQL connection target changed"),
        "{error}"
    );
}

#[test]
fn scoped_validation_observes_the_migration_lock_before_composition() {
    let Some(mut db) = Fixture::new() else { return };
    let (url, _) = bounded_url(&db);
    let mut transaction = db.admin.transaction().unwrap();
    transaction
        .query_one(
            "SELECT pg_advisory_xact_lock($1::integer,
         (SELECT oid::integer FROM pg_namespace WHERE nspname=$2))",
            &[&crate::SCHEMA_LOCK_CLASS, &db.schema],
        )
        .unwrap();
    let entered = Cell::new(false);
    let result = with_validated_postgres(&url, |_| {
        entered.set(true);
        Ok::<_, ApplicationError>(())
    });
    assert!(result.is_err());
    assert!(!entered.get());
    transaction.commit().unwrap();
    with_validated_postgres(&url, |_| Ok::<_, ApplicationError>(())).unwrap();
    assert!(mutation_lock_available(&mut db.admin));
}
