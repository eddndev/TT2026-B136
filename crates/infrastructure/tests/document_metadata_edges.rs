#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{CaseDocumentStore, DocumentMetadata, MetadataRevision};
use application::ApplicationError;
use metadata_database_support::{document, metadata, Fixture};
use postgres::{error::SqlState, Client, NoTls};

#[test]
fn direct_database_inserts_require_canonical_digest_contiguous_revision_and_existing_actor() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
    let mut runtime = Client::connect(&f.runtime_url, NoTls).unwrap();
    let before = f.snapshot();
    for (revision, digest, actor) in [
        (
            0,
            "pg_catalog.sha256(document_metadata_bytes(NULL,NULL,ARRAY[]::text[]))",
            f.owner.as_uuid(),
        ),
        (
            2,
            "pg_catalog.sha256(document_metadata_bytes(NULL,NULL,ARRAY[]::text[]))",
            f.owner.as_uuid(),
        ),
        (1, "decode(repeat('00',32),'hex')", f.owner.as_uuid()),
        (
            1,
            "pg_catalog.sha256(document_metadata_bytes(NULL,NULL,ARRAY[]::text[]))",
            uuid::Uuid::new_v4(),
        ),
    ] {
        let result=runtime.execute(&format!("INSERT INTO document_metadata_revisions(document_id,metadata_revision,tags,metadata_digest,changed_at,changed_by,changed_by_email) VALUES($1,$2,ARRAY[]::text[],{digest},'2025-01-01T00:00:00Z',$3,'actor@example.test')"),&[&record.id.as_uuid(),&i64::from(revision),&actor]);
        assert!(matches!(
            result.unwrap_err().code(),
            Some(&SqlState::CHECK_VIOLATION) | Some(&SqlState::FOREIGN_KEY_VIOLATION)
        ));
        assert_eq!(f.snapshot(), before);
    }
    store
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::unclassified(),
            metadata("Type", "Class", &[]),
            f.at,
        )
        .unwrap();
    let before = f.snapshot();
    let result = f.db.client.batch_execute(
        "UPDATE document_metadata_revisions SET changed_by_email='rewritten@example.test'",
    );
    assert_eq!(result.unwrap_err().code(), Some(&SqlState::CHECK_VIOLATION));
    for query in [
        "UPDATE document_metadata_revisions SET tags=tags WHERE false",
        "DELETE FROM document_metadata_revisions WHERE false",
        "TRUNCATE document_metadata_revisions",
    ] {
        assert_eq!(
            runtime.batch_execute(query).unwrap_err().code(),
            Some(&SqlState::INSUFFICIENT_PRIVILEGE)
        );
    }
    assert_eq!(f.snapshot(), before);
}

#[test]
fn an_exhausted_metadata_head_cannot_wrap_or_write_a_success_event() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store
        .insert_with_metadata(
            f.owner,
            f.case,
            record.clone(),
            DocumentMetadata::empty(),
            f.at,
        )
        .unwrap();
    f.db.client.batch_execute("ALTER TABLE document_metadata_revisions DISABLE TRIGGER document_metadata_preserve_history; UPDATE document_metadata_revisions SET metadata_revision=4294967295; ALTER TABLE document_metadata_revisions ENABLE TRIGGER document_metadata_preserve_history").unwrap();
    let before = f.snapshot();
    assert!(matches!(
        store.replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::new(u32::MAX),
            DocumentMetadata::empty(),
            f.at
        ),
        Err(ApplicationError::DocumentMetadataRevisionExhausted)
    ));
    assert_eq!(f.snapshot(), before);
    assert!(matches!(
        infrastructure::PostgresCaseDocumentStore::open(&f.runtime_url),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}
