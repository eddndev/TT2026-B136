#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{CaseDocumentStore, DocumentQuery, MetadataQuery, MetadataRevision};
use application::ApplicationError;
use metadata_database_support::{document, metadata, Fixture};

#[test]
fn insert_second_audit_and_deferred_commit_failures_roll_back_upload_and_replace() {
    for initial in [true, false] {
        for failure in ["row", "audit", "commit"] {
            let Some(mut f) = Fixture::new() else { return };
            let store = f.store();
            let record = document();
            if !initial {
                store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
            }
            if failure == "audit" {
                f.db.client.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_metadata_audit CHECK(action<>'document.metadata_changed')").unwrap();
            } else {
                let (constraint, timing, deferred) = if failure == "commit" {
                    ("CONSTRAINT ", "AFTER", "DEFERRABLE INITIALLY DEFERRED")
                } else {
                    ("", "BEFORE", "")
                };
                f.db.client.batch_execute(&format!("CREATE FUNCTION reject_metadata() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected metadata failure'; END $$; CREATE {constraint}TRIGGER reject_metadata {timing} INSERT ON document_metadata_revisions {deferred} FOR EACH ROW EXECUTE FUNCTION reject_metadata()")).unwrap();
            }
            let before = f.snapshot();
            let result = if initial {
                store
                    .insert_with_metadata(
                        f.owner,
                        f.case,
                        record,
                        metadata("Type", "Class", &[]),
                        f.at,
                    )
                    .map(|_| ())
            } else {
                store
                    .replace_metadata(
                        f.owner,
                        f.case,
                        record.id,
                        MetadataRevision::unclassified(),
                        metadata("Type", "Class", &[]),
                        f.at,
                    )
                    .map(|_| ())
            };
            assert!(
                matches!(result, Err(ApplicationError::Port(_))),
                "initial={initial},failure={failure}"
            );
            assert_eq!(f.snapshot(), before, "initial={initial},failure={failure}");
        }
    }
}

#[test]
fn metadata_reads_do_not_return_results_when_their_audit_fails() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store
        .insert_with_metadata(
            f.owner,
            f.case,
            record.clone(),
            metadata("Type", "Class", &[]),
            f.at,
        )
        .unwrap();
    f.db.client.batch_execute("ALTER TABLE audit_events ADD CONSTRAINT reject_metadata_reads CHECK(action NOT IN ('document.metadata_read','document.metadata_history_listed','document.read','document.listed'))").unwrap();
    let before = f.snapshot();
    let results = [
        store
            .get_metadata(f.owner, f.case, record.id, f.at)
            .map(|_| ()),
        store
            .metadata_history(
                f.owner,
                f.case,
                record.id,
                MetadataQuery::new(1, None).unwrap(),
                f.at,
            )
            .map(|_| ()),
        store
            .get_overview(f.owner, f.case, record.id, f.at)
            .map(|_| ()),
        store
            .list(
                f.owner,
                f.case,
                DocumentQuery::new(1, 0, None, None).unwrap(),
                f.at,
            )
            .map(|_| ()),
    ];
    assert!(results
        .into_iter()
        .all(|r| matches!(r, Err(ApplicationError::Port(_)))));
    assert_eq!(f.snapshot(), before);
}

#[test]
fn noncanonical_stored_metadata_is_rejected_without_normalization_or_audit() {
    for corruption in [
        "document_type=' Type'",
        "tags=ARRAY['z','a']",
        "metadata_digest='\\x00'",
        "metadata_digest=decode(repeat('00',32),'hex')",
        "changed_at='2025-01-01T01:00:00.123456789+01:00'",
    ] {
        let Some(mut f) = Fixture::new() else { return };
        let store = f.store();
        let record = document();
        store
            .insert_with_metadata(
                f.owner,
                f.case,
                record.clone(),
                metadata("Type", "Class", &["a"]),
                f.at,
            )
            .unwrap();
        f.db.client.batch_execute("ALTER TABLE document_metadata_revisions DISABLE TRIGGER document_metadata_preserve_history; ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_canonical; ALTER TABLE document_metadata_revisions DROP CONSTRAINT document_metadata_digest").unwrap();
        f.db.client
            .batch_execute(&format!(
                "UPDATE document_metadata_revisions SET {corruption}"
            ))
            .unwrap();
        let before = f.snapshot();
        for result in [
            store
                .get_metadata(f.owner, f.case, record.id, f.at)
                .map(|_| ()),
            store
                .metadata_history(
                    f.owner,
                    f.case,
                    record.id,
                    MetadataQuery::new(10, None).unwrap(),
                    f.at,
                )
                .map(|_| ()),
            store
                .get_overview(f.owner, f.case, record.id, f.at)
                .map(|_| ()),
            store
                .list(
                    f.owner,
                    f.case,
                    DocumentQuery::new(10, 0, None, None).unwrap(),
                    f.at,
                )
                .map(|_| ()),
        ] {
            assert!(
                matches!(
                    result,
                    Err(ApplicationError::StoredDocumentMetadataInconsistent(_))
                ),
                "accepted {corruption}"
            );
        }
        assert_eq!(f.snapshot(), before);
    }
}
