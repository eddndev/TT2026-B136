//! Backfill of account and invitation display names on existing databases:
//! docs/adr/0087-account-display-names.md.
#[path = "case_administration_support/mod.rs"]
mod case_administration_support;

use application::identity::{invitations::*, Principal};
use case_administration_support::Fixture;
use domain::{clock::OffsetDateTime, crypto::Sha256Digest, identity::Role};
use infrastructure::PostgresInvitationStore;
use serde_json::Value;

fn issue(db: &Fixture) -> InvitationSummary {
    let store = PostgresInvitationStore::open(&db.runtime_url).unwrap();
    let actor = InvitationActor {
        principal: Principal {
            id: db.owner,
            email: "owner@example.test".into(),
            role: Role::Owner,
        },
        revision: 0,
        auth_generation: 0,
    };
    let request = InvitationIssueRequest {
        operation_id: Uuid::new_v4(),
        email: "invited.person@example.test".into(),
        display_name: "Persona Invitada".into(),
        role: Role::Litigator,
    };
    store
        .issue(
            actor,
            IssueInvitation {
                request,
                token_digest: Sha256Digest::from_array([1; 32]),
                protected_token: vec![3; 80],
                at: OffsetDateTime::now_utc(),
            },
        )
        .unwrap()
}

fn names(db: &mut Fixture, table: &str) -> Vec<(String, String)> {
    let sql = format!("SELECT email,display_name FROM {table} ORDER BY email COLLATE \"C\"");
    db.admin
        .query(&sql, &[])
        .unwrap()
        .iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect()
}

fn last_column(db: &mut Fixture, table: &str) -> String {
    db.admin
        .query_one(
            "SELECT attname::text FROM pg_attribute WHERE attrelid=$1::text::regclass
            AND attnum>0 AND NOT attisdropped ORDER BY attnum DESC LIMIT 1",
            &[&table],
        )
        .unwrap()
        .get(0)
}

fn snapshot(db: &mut Fixture) -> Value {
    db.admin
        .query_one(
            "SELECT jsonb_build_object(
            'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
            'invitations',(SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM member_invitations i))",
            &[],
        )
        .unwrap()
        .get(0)
}

#[test]
fn existing_accounts_and_invitations_receive_the_email_local_part_once() {
    let Some(mut db) = Fixture::new() else { return };
    issue(&db);
    db.admin
        .batch_execute(
            "ALTER TABLE users DROP COLUMN display_name;
            ALTER TABLE member_invitations DROP COLUMN display_name;",
        )
        .unwrap();
    db.migrate();
    assert_eq!(
        names(&mut db, "users"),
        [("owner@example.test".to_owned(), "owner".to_owned())]
    );
    assert_eq!(
        names(&mut db, "member_invitations"),
        [(
            "invited.person@example.test".to_owned(),
            "invited.person".to_owned()
        )]
    );
    assert_eq!(last_column(&mut db, "users"), "display_name");
    assert_eq!(last_column(&mut db, "member_invitations"), "display_name");
    let before = snapshot(&mut db);
    db.migrate();
    assert_eq!(snapshot(&mut db), before);
    PostgresInvitationStore::open(&db.runtime_url).unwrap();
}

#[test]
fn the_placeholder_is_trimmed_bounded_and_never_empty() {
    let Some(mut db) = Fixture::new() else { return };
    db.admin
        .batch_execute(
            "ALTER TABLE users DROP COLUMN display_name;
            INSERT INTO users(id,email,password_hash,role,protected_totp_secret,recovery_codes)
            VALUES(gen_random_uuid(),'@nobody.test','fixture','litigator','\\x00','{}'),
            (gen_random_uuid(),repeat('a',119)||' bcd@example.test','fixture','paralegal','\\x00','{}')",
        )
        .unwrap();
    db.migrate();
    let long = "a".repeat(119);
    assert_eq!(
        names(&mut db, "users"),
        [
            ("@nobody.test".to_owned(), "cuenta".to_owned()),
            (format!("{long} bcd@example.test"), long.clone()),
            ("owner@example.test".to_owned(), "owner".to_owned()),
        ]
    );
}
