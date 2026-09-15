#[allow(dead_code)]
mod participant_database_support;

use application::participants::{
    DirectoryStatus, ParticipantHistoryQuery, ParticipantId, ParticipantRevision, ParticipantStore,
    ParticipantValues,
};
use application::ApplicationError;
use infrastructure::{PostgresParticipantStore, RingSha256Hasher};
use participant_database_support::Fixture;
use std::sync::Arc;

fn values(name: &str) -> ParticipantValues {
    ParticipantValues::new(
        name,
        "Witness",
        Some("Office"),
        None,
        DirectoryStatus::Active,
    )
    .unwrap()
}

#[test]
fn participant_replacements_capture_provenance_and_status_preserves_current_text() {
    let Some(mut f) = Fixture::new() else { return };
    let store =
        PostgresParticipantStore::open(&f.runtime_url, Arc::new(RingSha256Hasher::new())).unwrap();
    let id = ParticipantId::new();
    let first = store
        .create(f.owner, f.case, id, values("Ana"), f.at)
        .unwrap();
    assert_eq!(first.revision.get(), 1);
    assert_eq!(first.changed_by.id, f.owner);
    assert_eq!(first.changed_by.email, "owner@example.test");
    assert_eq!(first.changed_at, f.at);
    f.admin
        .execute(
            "UPDATE users SET email='new@example.test' WHERE id=$1",
            &[&f.owner.as_uuid()],
        )
        .unwrap();
    let second = store
        .replace(f.owner, f.case, id, first.revision, values("Updated"), f.at)
        .unwrap();
    let archived = store
        .change_status(
            f.owner,
            f.case,
            id,
            second.revision,
            DirectoryStatus::Archived,
            f.at,
        )
        .unwrap();
    assert_eq!(archived.values.display_name(), "Updated");
    assert_eq!(
        archived.values.directory_status(),
        DirectoryStatus::Archived
    );
    assert_eq!(archived.changed_by.email, "new@example.test");
    let history = store
        .history(
            f.owner,
            f.case,
            id,
            ParticipantHistoryQuery::new(2, None).unwrap(),
            f.at,
        )
        .unwrap();
    assert_eq!(history.revisions, vec![archived.clone(), second]);
    assert!(history.has_more);
    let old = store
        .history(
            f.owner,
            f.case,
            id,
            ParticipantHistoryQuery::new(2, Some(2)).unwrap(),
            f.at,
        )
        .unwrap();
    assert_eq!(old.revisions, vec![first]);
    assert!(!old.has_more);
    assert_eq!(store.get(f.owner, f.case, id, f.at).unwrap(), archived);
}

#[test]
fn stale_and_identical_mutations_have_explicit_revision_semantics() {
    let Some(mut f) = Fixture::new() else { return };
    let store =
        PostgresParticipantStore::open(&f.runtime_url, Arc::new(RingSha256Hasher::new())).unwrap();
    let id = ParticipantId::new();
    store
        .create(f.owner, f.case, id, values("Same"), f.at)
        .unwrap();
    let r2 = store
        .change_status(
            f.owner,
            f.case,
            id,
            ParticipantRevision::initial(),
            DirectoryStatus::Active,
            f.at,
        )
        .unwrap();
    assert_eq!(r2.revision.get(), 2);
    let before = f.snapshot();
    assert!(matches!(
        store.replace(
            f.owner,
            f.case,
            id,
            ParticipantRevision::initial(),
            values("Stale"),
            f.at
        ),
        Err(ApplicationError::ParticipantRevisionConflict)
    ));
    assert_eq!(f.snapshot(), before);
    let r3 = store
        .replace(f.owner, f.case, id, r2.revision, values("Same"), f.at)
        .unwrap();
    assert_eq!(r3.revision.get(), 3);
    assert_eq!(r3.values_digest, r2.values_digest);
}

#[test]
fn create_rejects_archived_values_and_names_never_grant_account_access() {
    let Some(mut f) = Fixture::new() else { return };
    let store =
        PostgresParticipantStore::open(&f.runtime_url, Arc::new(RingSha256Hasher::new())).unwrap();
    let before = f.snapshot();
    assert!(store
        .create(
            f.owner,
            f.case,
            ParticipantId::new(),
            values("Judge").with_directory_status(DirectoryStatus::Archived),
            f.at
        )
        .is_err());
    assert_eq!(f.snapshot(), before);
    for _ in 0..2 {
        store
            .create(
                f.owner,
                f.case,
                ParticipantId::new(),
                values("Same person label"),
                f.at,
            )
            .unwrap();
    }
    let after = f.snapshot();
    for key in ["users", "members", "documents", "metadata"] {
        assert_eq!(after[key], before[key]);
    }
    assert_eq!(after["roots"].as_array().unwrap().len(), 2);
}
