//! Invitee display names written by the Owner at issue:
//! docs/adr/0087-account-display-names.md.
use super::{actor, digest, finish, fixture, issue, pending, prepared};
use application::{identity::invitations::*, ApplicationError};
use domain::identity::DisplayName;
use infrastructure::PostgresInvitationStore;

fn stored_name(db: &mut super::Fixture, sql: &str) -> String {
    db.admin.query_one(sql, &[]).unwrap().get::<_, String>(0)
}

#[test]
fn issue_stores_the_name_and_binds_it_to_the_operation() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let op = Uuid::new_v4();
    let first = store
        .issue(actor(&db), issue(op, "invited@example.test"))
        .unwrap();
    assert_eq!(first.display_name.as_str(), "Test Invitee");
    assert_eq!(
        stored_name(&mut db, "SELECT display_name FROM member_invitations"),
        "Test Invitee"
    );
    assert_eq!(
        store
            .issue(actor(&db), issue(op, "invited@example.test"))
            .unwrap(),
        first
    );
    let mut renamed = issue(op, "invited@example.test");
    renamed.request.display_name = "Otra Persona".into();
    assert!(matches!(
        store.issue(actor(&db), renamed),
        Err(ApplicationError::ConcurrentModification)
    ));
    assert_eq!(store.get(actor(&db), first.id).unwrap(), first);
    let candidate = store.inspect(digest(1)).unwrap().unwrap();
    assert_eq!(candidate.display_name, first.display_name);
}

#[test]
fn acceptance_requires_and_keeps_the_invitation_name() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    let p = store.save_pending(prepared(first.id, pending())).unwrap();
    let mut other = finish(first.id, &p);
    other.user.display_name = DisplayName::new("Otra Persona").unwrap();
    assert!(matches!(
        store.finish(other),
        Err(ApplicationError::InvalidCredentials)
    ));
    let count: i64 = db
        .admin
        .query_one(
            "SELECT count(*) FROM users WHERE email='invited@example.test'",
            &[],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 0);
    store.finish(finish(first.id, &p)).unwrap();
    assert_eq!(
        stored_name(
            &mut db,
            "SELECT display_name FROM users WHERE email='invited@example.test'"
        ),
        "Test Invitee"
    );
}

#[test]
fn invitation_names_are_required_immutable_and_fail_closed() {
    let mut db = fixture();
    let row = db
        .admin
        .query_one(
            "SELECT format_type(a.atttypid,a.atttypmod),a.attnotnull,
            EXISTS(SELECT 1 FROM pg_attrdef d WHERE d.adrelid=a.attrelid AND d.adnum=a.attnum),
            EXISTS(SELECT 1 FROM pg_constraint c WHERE c.conrelid=a.attrelid
                AND c.conname='member_invitations_display_name_check' AND c.convalidated),
            has_column_privilege($1,'member_invitations','display_name','UPDATE')
        FROM pg_attribute a WHERE a.attrelid='member_invitations'::regclass
            AND a.attname='display_name' AND NOT a.attisdropped",
            &[&db.role],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "text");
    assert!(row.get::<_, bool>(1));
    assert!(!row.get::<_, bool>(2));
    assert!(row.get::<_, bool>(3));
    assert!(!row.get::<_, bool>(4));
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let first = store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    db.admin
        .batch_execute("UPDATE member_invitations SET display_name='a'||chr(8238)||'b'")
        .unwrap();
    assert!(matches!(
        store.get(actor(&db), first.id),
        Err(ApplicationError::Port(_))
    ));
    db.admin
        .batch_execute("UPDATE member_invitations SET display_name='Test Invitee'")
        .unwrap();
    db.admin
        .batch_execute(&format!(
            "GRANT UPDATE(display_name) ON member_invitations TO {}",
            db.role
        ))
        .unwrap();
    assert!(PostgresInvitationStore::open(&db.runtime_url).is_err());
}

#[test]
fn startup_rejects_a_stored_invitation_name_that_is_not_canonical() {
    let mut db = fixture();
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    store
        .issue(actor(&db), issue(Uuid::new_v4(), "invited@example.test"))
        .unwrap();
    // A no-break space passes the database CHECK, which trims only ASCII spaces.
    db.admin
        .batch_execute("UPDATE member_invitations SET display_name='Ana'||chr(160)")
        .unwrap();
    let reopened = PostgresInvitationStore::open(&db.runtime_url).map(|_| ());
    assert!(
        format!("{reopened:?}").contains("invalid invitation display name"),
        "{reopened:?}"
    );
}
