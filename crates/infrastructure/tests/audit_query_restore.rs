use crate::audit_query_support::*;

#[test]
fn backup_restore_keeps_projection_selected_history_and_chain_then_allows_successor() {
    let Some(mut db) = Fixture::new() else { return };
    append(
        &db,
        "archive actor",
        "archive.op",
        "archive resource",
        db.at,
    );
    append(
        &db,
        "archive actor",
        "archive.op",
        "archive resource",
        db.at + Duration::nanoseconds(1),
    );
    let request = AuditEventQuery::new(
        db.at,
        db.at + Duration::seconds(1),
        Some("archive actor"),
        Some("archive.op"),
        None,
        20,
        None,
    )
    .unwrap();
    let page = store(&db).read(&owner(&db), &request, db.at).unwrap();
    let before = original_columns(&mut db);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("audit-query.dump");
    let result = std::process::Command::new("pg_dump")
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
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", db.schema))
        .unwrap();
    let result = std::process::Command::new("pg_restore")
        .arg("--exit-on-error")
        .arg("--dbname")
        .arg(&db.admin_url)
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(original_columns(&mut db), before);
    db.migrate();
    assert_eq!(original_columns(&mut db), before);
    let restored = store(&db).read(&owner(&db), &request, db.at).unwrap();
    assert_eq!(restored.events, page.events);
    assert!(!restored.has_more);
    let sequence = append(
        &db,
        "archive actor",
        "after.restore",
        "archive resource",
        db.at,
    );
    assert_eq!(sequence, 4);
    assert_chain(&db);
}
