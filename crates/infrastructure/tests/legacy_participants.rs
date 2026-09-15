#[allow(dead_code)]
mod legacy_database_support;

use application::documents::CaseDocumentStore;
use application::participants::{
    DirectoryStatus, ParticipantId, ParticipantStore, ParticipantValues,
};
use domain::identity::UserId;
use infrastructure::{PostgresParticipantStore, RingSha256Hasher};
use legacy_database_support::{Database, Source};
use std::sync::Arc;

#[test]
fn initial_import_rejects_a_participant_directory_even_without_audit_events() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    let id = uuid::Uuid::new_v4();
    let mut tx = db.client.transaction().unwrap();
    tx.execute(
        "INSERT INTO case_participants(id,case_id) VALUES($1,$2)",
        &[&id, &source.case_id.as_uuid()],
    )
    .unwrap();
    tx.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,directory_status,values_digest,changed_at,changed_by,changed_by_email)
        SELECT $1,1,'Existing','Witness','active',pg_catalog.sha256(participant_values_bytes('Existing','Witness',NULL,NULL,'active')),'2025-01-01T00:00:00Z',created_by,'old@example.test' FROM cases WHERE id=$2",
        &[&id,&source.case_id.as_uuid()]).unwrap();
    tx.commit().unwrap();
    let before = db.stored_state();
    assert!(source.inspect().check_target(&db.url).is_err());
    assert!(source.inspect().apply(&db.url).is_err());
    assert_eq!(db.stored_state(), before);
    source.assert_unmarked();
}

#[test]
fn completed_import_reconciliation_preserves_later_participants_and_exact_legacy_prefix() {
    let Some(mut db) = Database::new() else {
        return;
    };
    let source = Source::new();
    db.seed_case(source.case_id);
    source.inspect().apply(&db.url).unwrap();
    let url = db.restricted_url();
    let owner = UserId::from_uuid(
        db.client
            .query_one(
                "SELECT created_by FROM cases WHERE id=$1",
                &[&source.case_id.as_uuid()],
            )
            .unwrap()
            .get(0),
    );
    let store = PostgresParticipantStore::open(&url, Arc::new(RingSha256Hasher::new())).unwrap();
    let id = ParticipantId::new();
    let at = time::OffsetDateTime::now_utc();
    let first = store
        .create(
            owner,
            source.case_id,
            id,
            ParticipantValues::new("Recorded", "Witness", None, None, DirectoryStatus::Active)
                .unwrap(),
            at,
        )
        .unwrap();
    store
        .change_status(
            owner,
            source.case_id,
            id,
            first.revision,
            DirectoryStatus::Archived,
            at,
        )
        .unwrap();
    let before = db.stored_state();
    let participants: serde_json::Value = db
        .client
        .query_one(
            "SELECT jsonb_agg(to_jsonb(r) ORDER BY revision) FROM case_participant_revisions r",
            &[],
        )
        .unwrap()
        .get(0);
    source.inspect().check_target(&db.url).unwrap();
    source.inspect().apply(&db.url).unwrap();
    infrastructure::legacy::require_completed_import(source.dir.path(), &url).unwrap();
    assert_eq!(db.stored_state(), before);
    assert_eq!(
        db.client
            .query_one(
                "SELECT jsonb_agg(to_jsonb(r) ORDER BY revision) FROM case_participant_revisions r",
                &[]
            )
            .unwrap()
            .get::<_, serde_json::Value>(0),
        participants
    );
    let audit = infrastructure::PostgresCaseDocumentStore::open(&url)
        .unwrap()
        .audit_entries(owner)
        .unwrap();
    assert_eq!(audit[..source.entries.len()], source.entries);
}
