use crate::metadata_database_support;
use application::documents::CaseDocumentStore;
use infrastructure::PostgresCaseDocumentStore;
use metadata_database_support::{document, Fixture};
use postgres::{Client, NoTls};

#[test]
fn upload_origin_runtime_denies_changes_and_rejects_altered_schema_or_grants() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store.insert(f.owner, f.case, record, f.at).unwrap();
    let mut runtime = Client::connect(&f.runtime_url, NoTls).unwrap();
    for sql in [
        "UPDATE document_upload_origins SET actor_id=actor_id",
        "DELETE FROM document_upload_origins",
        "TRUNCATE document_upload_origins",
    ] {
        assert_eq!(
            runtime.batch_execute(sql).unwrap_err().code(),
            Some(&postgres::error::SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    for (alter, restore) in [
        (
            "ALTER TABLE document_upload_origins ADD COLUMN extra boolean".to_owned(),
            "ALTER TABLE document_upload_origins DROP COLUMN extra".to_owned(),
        ),
        (
            "ALTER TABLE document_upload_origins DISABLE TRIGGER document_upload_origin_immutable"
                .to_owned(),
            "ALTER TABLE document_upload_origins ENABLE TRIGGER document_upload_origin_immutable"
                .to_owned(),
        ),
        (
            format!("GRANT UPDATE ON document_upload_origins TO {}", f.role),
            format!("REVOKE UPDATE ON document_upload_origins FROM {}", f.role),
        ),
    ] {
        f.db.client.batch_execute(&alter).unwrap();
        assert!(PostgresCaseDocumentStore::open(&f.runtime_url).is_err());
        f.db.client.batch_execute(&restore).unwrap();
        PostgresCaseDocumentStore::open(&f.runtime_url).unwrap();
    }
    f.db.client
        .batch_execute(
            "ALTER TABLE document_upload_origins DISABLE TRIGGER document_upload_origin_immutable;
             UPDATE document_upload_origins SET recorded_at_nanoseconds=0;
             ALTER TABLE document_upload_origins ENABLE TRIGGER document_upload_origin_immutable",
        )
        .unwrap();
    assert!(PostgresCaseDocumentStore::open(&f.runtime_url).is_err());
}

#[test]
fn restored_upload_origins_preserve_rows_and_open_with_restricted_runtime() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    store.insert(f.owner, f.case, document(), f.at).unwrap();
    let before = snapshot(&mut f);
    drop(store);
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("document-upload-origins.dump");
    let output = std::process::Command::new("pg_dump")
        .arg("--dbname")
        .arg(&f.db.url)
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
    f.db.control
        .batch_execute(&format!("DROP SCHEMA {} CASCADE", f.db.schema))
        .unwrap();
    let output = std::process::Command::new("pg_restore")
        .arg("--exit-on-error")
        .arg("--dbname")
        .arg(&f.db.url)
        .arg(&dump)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    PostgresCaseDocumentStore::open(&f.runtime_url).unwrap();
    assert_eq!(snapshot(&mut f), before);
}

fn snapshot(f: &mut Fixture) -> serde_json::Value {
    let mut value = f.snapshot();
    value["origins"] =
        f.db.client
            .query_one(
                "SELECT jsonb_agg(to_jsonb(o) ORDER BY document_id) FROM document_upload_origins o",
                &[],
            )
            .unwrap()
            .get(0);
    value
}
