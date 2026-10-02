use crate::member_support;
use application::members::{MemberStore, UserAccessChange};
use domain::identity::Role;
use member_support::{snapshot, store, user, Fixture};

#[test]
fn dump_restore_preserves_access_generations_memberships_and_runtime_guards() {
    let Some(mut db) = Fixture::new() else { return };
    let target = user(&mut db, "restored@example.test", "litigator", true, true);
    let members = store(&db);
    members
        .change_access(
            db.owner,
            target,
            UserAccessChange::new(0, Role::Paralegal, false).unwrap(),
            db.at,
        )
        .unwrap();
    let before = snapshot(&mut db);
    drop(members);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("members.dump");
    let output = std::process::Command::new("pg_dump")
        .arg("--dbname")
        .arg(&db.admin_url)
        .arg("--schema")
        .arg(&db.schema)
        .arg("--format=custom")
        .arg("--file")
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .arg("--exit-on-error")
        .arg("--dbname")
        .arg(&db.admin_url)
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(snapshot(&mut db), before);
    let members = store(&db);
    let result = members
        .change_access(
            db.owner,
            target,
            UserAccessChange::new(1, Role::Paralegal, true).unwrap(),
            db.at,
        )
        .unwrap();
    assert_eq!(result.revision, 2);
    assert_eq!(
        db.admin
            .query_one(
                "SELECT auth_generation FROM users WHERE id=$1",
                &[&target.as_uuid()]
            )
            .unwrap()
            .get::<_, i64>(0),
        2
    );
    assert!(db
        .runtime()
        .batch_execute("UPDATE users SET auth_generation=0")
        .is_err());
}
