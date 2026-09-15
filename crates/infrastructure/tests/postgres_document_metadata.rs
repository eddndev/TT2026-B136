#[allow(dead_code)]
mod metadata_database_support;

use application::documents::{
    CaseDocumentStore, DocumentMetadata, MetadataQuery, MetadataRevision,
};
use application::ApplicationError;
use metadata_database_support::{document, metadata, Fixture};

#[test]
fn current_and_history_preserve_revision_zero_and_historical_actor_identity() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let record = document();
    let uploaded = store.insert(f.owner, f.case, record.clone(), f.at).unwrap();
    assert_eq!(uploaded.current_metadata.metadata_revision.get(), 0);
    assert_eq!(uploaded.current_metadata.values, DocumentMetadata::empty());
    assert_eq!(
        store
            .get_metadata(f.owner, f.case, record.id, f.at)
            .unwrap(),
        uploaded.current_metadata
    );
    assert!(store
        .metadata_history(
            f.owner,
            f.case,
            record.id,
            MetadataQuery::new(10, None).unwrap(),
            f.at
        )
        .unwrap()
        .revisions
        .is_empty());
    let values = metadata(" Civil ", "Private", &["z", "a,b", "\u{e1}", "z"]);
    let changed = store
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::unclassified(),
            values.clone(),
            f.at,
        )
        .unwrap();
    assert_eq!(changed.metadata_revision.get(), 1);
    assert_eq!(changed.values, values);
    let actor_email = format!("{}@example.test", f.owner);
    f.db.client
        .execute(
            "UPDATE users SET email='renamed@example.test' WHERE id=$1",
            &[&f.owner.as_uuid()],
        )
        .unwrap();
    store
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            changed.metadata_revision,
            DocumentMetadata::empty(),
            f.at,
        )
        .unwrap();
    let history = store
        .metadata_history(
            f.owner,
            f.case,
            record.id,
            MetadataQuery::new(1, None).unwrap(),
            f.at,
        )
        .unwrap();
    assert!(history.has_more);
    assert_eq!(history.revisions[0].metadata_revision.get(), 2);
    assert_eq!(
        history.revisions[0].changed_by.email,
        "renamed@example.test"
    );
    let older = store
        .metadata_history(
            f.owner,
            f.case,
            record.id,
            MetadataQuery::new(10, Some(history.next_before_revision.unwrap().get())).unwrap(),
            f.at,
        )
        .unwrap();
    assert_eq!(older.revisions.len(), 1);
    let first = &older.revisions[0];
    assert_eq!(first.changed_by.id, f.owner);
    assert_eq!(first.changed_by.email, actor_email);
    assert_eq!(first.changed_at, f.at);
    assert_eq!(first.values, values);
    assert_eq!(
        first.metadata_digest,
        application::documents::metadata_digest(&infrastructure::RingSha256Hasher::new(), &values)
    );
    assert!(!older.has_more);
    assert_eq!(older.next_before_revision, None);
}

#[test]
fn classified_upload_commits_two_events_and_append_returns_current_metadata() {
    let Some(mut f) = Fixture::new() else { return };
    let store = f.store();
    let mut record = document();
    let values = metadata("Type", "Class", &["a"]);
    let overview = store
        .insert_with_metadata(f.owner, f.case, record.clone(), values.clone(), f.at)
        .unwrap();
    assert_eq!(overview.content.document.id, record.id);
    assert_eq!(overview.current_metadata.values, values);
    assert_eq!(overview.current_metadata.metadata_revision.get(), 1);
    let events = store.audit_entries(f.owner).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event.action, "document.uploaded");
    assert_eq!(events[1].event.action, "document.metadata_changed");
    assert_eq!(events[0].event.actor, events[1].event.actor);
    assert_eq!(events[0].event.timestamp, events[1].event.timestamp);
    assert_eq!(
        events[1].event.resource,
        format!(
            "case:{}:document:{}:metadata:1:sha256:{}",
            f.case,
            record.id,
            application::documents::metadata_digest(
                &infrastructure::RingSha256Hasher::new(),
                &values
            )
            .to_hex()
        )
    );
    let expected = record.version;
    record.version = expected.next().unwrap();
    record.vault.push(9);
    let appended = store
        .append(f.owner, f.case, expected, record.clone(), f.at)
        .unwrap();
    assert_eq!(appended.content.document.version, record.version);
    assert_eq!(appended.current_metadata, overview.current_metadata);
    let before = f.snapshot();
    assert!(matches!(
        store.replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::unclassified(),
            values,
            f.at
        ),
        Err(ApplicationError::DocumentMetadataConflict)
    ));
    assert_eq!(f.snapshot(), before);
    let cleared = store
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            MetadataRevision::new(1),
            DocumentMetadata::empty(),
            f.at,
        )
        .unwrap();
    assert_eq!(cleared.metadata_revision.get(), 2);
    let repeated = store
        .replace_metadata(
            f.owner,
            f.case,
            record.id,
            cleared.metadata_revision,
            DocumentMetadata::empty(),
            f.at,
        )
        .unwrap();
    assert_eq!(repeated.metadata_revision.get(), 3);
}
