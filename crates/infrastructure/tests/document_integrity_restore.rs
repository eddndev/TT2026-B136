mod document_content_support;
use application::document_integrity::DocumentIntegrityStore;
use document_content_support::Fixture;
use infrastructure::PostgresCaseDocumentStore;

#[test]
fn restored_integrity_inbox_preserves_captures_replay_and_runtime_guards() {
    let Some(mut f) = Fixture::new() else { return };
    let observation = f.observation();
    let receipt = f.store.record_rejection(&observation).unwrap();
    let incident = f
        .store
        .get(f.db.owner, receipt.incident_id, f.db.at)
        .unwrap();
    let before = snapshot(&mut f);
    drop(f.store);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("document-integrity.dump");
    let output = std::process::Command::new("pg_dump")
        .arg("--dbname")
        .arg(&f.db.admin_url)
        .arg("--schema")
        .arg(&f.db.schema)
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
    f.db.admin
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", f.db.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .arg("--exit-on-error")
        .arg("--dbname")
        .arg(&f.db.admin_url)
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    f.store = PostgresCaseDocumentStore::open(&f.db.runtime_url).unwrap();
    assert_eq!(snapshot(&mut f), before);
    assert_eq!(
        f.store
            .get(f.db.owner, receipt.incident_id, f.db.at)
            .unwrap(),
        incident
    );
    assert_eq!(f.store.record_rejection(&observation).unwrap(), receipt);
    assert!(f
        .db
        .runtime()
        .batch_execute("DELETE FROM document_integrity_incidents")
        .is_err());
}

fn snapshot(f: &mut Fixture) -> Vec<serde_json::Value> {
    [
        "documents",
        "document_series",
        "document_integrity_incidents",
        "audit_events",
    ]
    .iter()
    .map(|table| {
        f.db.admin
            .query_one(
                &format!(
                    "SELECT coalesce(jsonb_agg(row_data ORDER BY row_data::text),'[]'::jsonb)
             FROM (SELECT to_jsonb(t) row_data FROM {table} t) rows"
                ),
                &[],
            )
            .unwrap()
            .get(0)
    })
    .collect()
}
