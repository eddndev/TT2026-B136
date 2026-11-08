//! Stored account display names: docs/adr/0087-account-display-names.md.
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;

use application::identity::UserRepository;
use application::ApplicationError;
use case_administration_support::Fixture;
use domain::clock::OffsetDateTime;
use domain::identity::{DisplayName, Role};
use infrastructure::PostgresUserRepository;
use postgres::error::SqlState;

fn insert(db: &mut Fixture, email: &str, name_sql: &str) -> Result<u64, postgres::Error> {
    db.admin.execute(
        &format!(
            "INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes,
            display_name) VALUES(gen_random_uuid(),$1,'fixture','litigator','\\x00','{{}}',{name_sql})"
        ),
        &[&email],
    )
}

#[test]
fn the_catalog_rejects_untrimmed_empty_and_overlong_names() {
    let Some(mut db) = Fixture::new() else { return };
    for (email, name) in [
        ("long@example.test", "repeat('a',121)"),
        ("empty@example.test", "''"),
        ("padded@example.test", "' Ana'"),
    ] {
        let error = insert(&mut db, email, name).unwrap_err();
        assert_eq!(error.code(), Some(&SqlState::CHECK_VIOLATION), "{email}");
    }
    let repository = PostgresUserRepository::open(&db.runtime_url).unwrap();
    let mut wide = super::user("wide@example.test", Role::Litigator);
    wide.display_name = DisplayName::new(&"\u{e9}".repeat(120)).unwrap();
    repository
        .insert(wide.clone(), db.owner, OffsetDateTime::now_utc())
        .unwrap();
    let stored = repository
        .find_by_email("wide@example.test")
        .unwrap()
        .unwrap();
    assert_eq!(stored.display_name, wide.display_name);
}

#[test]
fn a_stored_name_with_format_characters_fails_closed() {
    let Some(mut db) = Fixture::new() else { return };
    let owner: String = db
        .admin
        .query_one(
            "SELECT display_name FROM users WHERE id=$1",
            &[&db.owner.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(owner, "owner", "existing rows keep the email local part");
    let repository = PostgresUserRepository::open(&db.runtime_url).unwrap();
    let named = super::user("named@example.test", Role::Litigator);
    repository
        .insert(named.clone(), db.owner, OffsetDateTime::now_utc())
        .unwrap();
    db.admin
        .execute(
            "UPDATE users SET display_name='a'||chr(8238)||'b' WHERE id=$1",
            &[&named.id.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        repository.find_by_id(named.id),
        Err(ApplicationError::Port(reason)) if reason == "stored display name is invalid"
    ));
    let reopened = PostgresUserRepository::open(&db.runtime_url).map(|_| ());
    assert!(
        format!("{reopened:?}").contains("invalid user display name"),
        "{reopened:?}"
    );
}
