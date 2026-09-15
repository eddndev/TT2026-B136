#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{
    CaseDocumentStore, DocumentMetadataFilter, DocumentQuery, MetadataRevision,
};
use metadata_database_support::{document, metadata, Fixture};

#[test]
fn filters_use_current_metadata_and_content_heads_before_pagination() {
    let Some(f) = Fixture::new() else { return };
    let store = f.store();
    let first = document();
    store
        .insert_with_metadata(
            f.owner,
            f.case,
            first.clone(),
            metadata("Old", "Private", &["gone"]),
            f.at,
        )
        .unwrap();
    store
        .replace_metadata(
            f.owner,
            f.case,
            first.id,
            MetadataRevision::new(1),
            metadata("Type", "Civil", &["a,b", "%_"]),
            f.at,
        )
        .unwrap();
    let mut next = first.clone();
    next.version = first.version.next().unwrap();
    next.name = "latest.txt".into();
    store
        .append(f.owner, f.case, first.version, next.clone(), f.at)
        .unwrap();
    let plain = document();
    store.insert(f.owner, f.case, plain.clone(), f.at).unwrap();
    for filter in [
        DocumentMetadataFilter::new(Some("Old"), None, None).unwrap(),
        DocumentMetadataFilter::new(None, None, Some("gone")).unwrap(),
        DocumentMetadataFilter::new(Some("type"), None, None).unwrap(),
        DocumentMetadataFilter::new(Some("Type"), Some("Private"), None).unwrap(),
        DocumentMetadataFilter::new(None, None, Some("a")).unwrap(),
    ] {
        assert!(store
            .list(
                f.owner,
                f.case,
                DocumentQuery::new(1, 0, None, None)
                    .unwrap()
                    .with_metadata_filter(filter),
                f.at
            )
            .unwrap()
            .documents
            .is_empty());
    }
    let filter = DocumentMetadataFilter::new(Some("Type"), Some("Civil"), Some("%_")).unwrap();
    let page = store
        .list(
            f.owner,
            f.case,
            DocumentQuery::new(1, 0, Some("latest"), None)
                .unwrap()
                .with_metadata_filter(filter.clone()),
            f.at,
        )
        .unwrap();
    assert_eq!(page.documents.len(), 1);
    assert!(!page.has_more);
    assert_eq!(page.documents[0].content.document.version, next.version);
    assert_eq!(
        page.documents[0].current_metadata.metadata_revision.get(),
        2
    );
    assert!(store
        .list(
            f.owner,
            f.case,
            DocumentQuery::new(1, 0, Some("evidence"), None)
                .unwrap()
                .with_metadata_filter(filter),
            f.at
        )
        .unwrap()
        .documents
        .is_empty());
    let plain_overview = store.get_overview(f.owner, f.case, plain.id, f.at).unwrap();
    assert_eq!(plain_overview.current_metadata.metadata_revision.get(), 0);
}

#[test]
fn metadata_queries_never_decode_corrupted_ciphertext_or_historical_evidence() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    store
        .insert_with_metadata(
            f.owner,
            f.case,
            record.clone(),
            metadata("Type", "Civil", &["a"]),
            f.at,
        )
        .unwrap();
    f.db.client
        .batch_execute("ALTER TABLE documents DISABLE TRIGGER documents_preserve_evidence")
        .unwrap();
    f.db.client
        .execute(
            "UPDATE documents SET vault='\\x00',evidence='{}' WHERE id=$1",
            &[&record.id.as_uuid()],
        )
        .unwrap();
    f.db.client
        .batch_execute("ALTER TABLE documents ENABLE TRIGGER documents_preserve_evidence")
        .unwrap();
    let result = store
        .get_overview(f.owner, f.case, record.id, f.at)
        .unwrap();
    assert!(result.content.document.sealed);
    assert_eq!(result.current_metadata.metadata_revision.get(), 1);
    assert_eq!(
        store
            .list(
                f.owner,
                f.case,
                DocumentQuery::new(10, 0, None, None).unwrap(),
                f.at
            )
            .unwrap()
            .documents
            .len(),
        1
    );
}
