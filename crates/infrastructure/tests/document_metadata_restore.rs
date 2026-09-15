#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{metadata_digest, CaseDocumentStore, MetadataRevision};
use metadata_database_support::{document, metadata, Fixture};
use postgres::{Client, NoTls};

#[test]
fn metadata_canonical_functions_and_insert_checks_work_with_an_empty_search_path() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
    let values = metadata("Type", "Class", &["a,b", "\u{e1}"]);
    let digest = metadata_digest(&infrastructure::RingSha256Hasher::new(), &values);
    let mut client = Client::connect(&f.runtime_url, NoTls).unwrap();
    client.batch_execute("SET search_path=''").unwrap();
    let bytes: Vec<u8> = client
        .query_one(
            &format!("SELECT {}.document_metadata_bytes($1,$2,$3)", f.db.schema),
            &[
                &values.document_type(),
                &values.classification(),
                &values.tags(),
            ],
        )
        .unwrap()
        .get(0);
    assert_eq!(bytes, values.canonical_bytes());
    for revision in 1_i64..=2 {
        client.execute(&format!(
            "INSERT INTO {}.document_metadata_revisions(document_id,metadata_revision,document_type,classification,tags,metadata_digest,changed_at,changed_by,changed_by_email)
             VALUES($1,$2,$3,$4,$5,$6,'2025-01-01T00:00:00Z',$7,'author@example.test')", f.db.schema),
            &[&record.id.as_uuid(), &revision, &values.document_type(), &values.classification(),
                &values.tags(), &&digest.as_bytes()[..], &f.owner.as_uuid()],
        ).unwrap();
    }
    let current = store
        .get_metadata(f.owner, f.case, record.id, f.at)
        .unwrap();
    assert_eq!(current.metadata_revision.get(), 2);
    assert_eq!(current.values, values);
    let before = f.snapshot();
    let result = client.execute(&format!(
        "INSERT INTO {}.document_metadata_revisions(document_id,metadata_revision,tags,metadata_digest,changed_at,changed_by,changed_by_email)
         VALUES($1,3,ARRAY[]::text[],decode(repeat('00',32),'hex'),'2025-01-01T00:00:00Z',$2,'author@example.test')",f.db.schema),
        &[&record.id.as_uuid(),&f.owner.as_uuid()],
    );
    assert_eq!(
        result.unwrap_err().code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
    assert_eq!(f.snapshot(), before);
}

#[test]
fn pg_dump_and_pg_restore_preserve_metadata_rows_checks_and_runtime_access() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    let values = metadata("Type", "Class", &["a,b", "\u{e1}"]);
    store
        .insert_with_metadata(f.owner, f.case, record.clone(), values.clone(), f.at)
        .unwrap();
    store
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::new(1),
            values,
            f.at,
        )
        .unwrap();
    drop(store);
    let before = f.snapshot();
    let directory = tempfile::tempdir().unwrap();
    let dump = directory.path().join("classification.dump");
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
    assert_eq!(f.snapshot(), before);
    let reopened = f.store();
    assert_eq!(
        reopened
            .get_metadata(f.owner, f.case, record.id, f.at)
            .unwrap()
            .metadata_revision
            .get(),
        2
    );
    assert!(reopened
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::new(2),
            metadata("Restored", "Class", &[]),
            f.at
        )
        .is_ok());
}
