mod document_content_support;
#[allow(dead_code)]
mod document_store_support;

use application::document_integrity::DocumentIntegrityFailure;
use application::documents::{CaseDocumentStore, DocumentAction, VersionSelection};
use application::ApplicationError;
use document_content_support::{record, Fixture};
use domain::crypto::{DocumentId, DocumentVersion};

#[test]
fn exact_content_access_survives_a_new_head_and_later_sealing() {
    let Some(mut f) = Fixture::new() else { return };
    let selected = f
        .store
        .load(
            f.db.owner,
            f.db.case,
            f.record.id,
            VersionSelection::Exact(f.record.version),
            DocumentAction::ReadContent,
        )
        .unwrap();
    let second = record(
        f.record.id,
        DocumentVersion::new(2).unwrap(),
        b"new content",
    );
    f.store
        .append(f.db.owner, f.db.case, f.record.version, second, f.db.at)
        .unwrap();
    let mut sealed = f.record.clone();
    sealed.seal(document_store_support::evidence()).unwrap();
    f.store
        .seal(f.db.owner, f.db.case, sealed, f.db.at)
        .unwrap();
    assert_eq!(f.content_events(), 0);
    f.store
        .record_access(
            f.db.owner,
            f.db.case,
            &selected,
            DocumentAction::ReadContent,
            f.db.at,
        )
        .unwrap();
    assert_eq!(f.content_events(), 1);
    let resource: String =
        f.db.admin
            .query_one(
                "SELECT resource FROM audit_events WHERE action='document.content_authorized'",
                &[],
            )
            .unwrap()
            .get(0);
    assert!(resource.contains(&format!("document:{}:version:1:", f.record.id)));
}

#[test]
fn content_checks_current_membership_and_rejects_changed_snapshots_without_success() {
    let Some(mut f) = Fixture::new() else { return };
    for role in ["litigator", "paralegal"] {
        let actor = f.db.user(role, true);
        let loaded = f
            .store
            .load(
                actor,
                f.db.case,
                f.record.id,
                VersionSelection::Exact(f.record.version),
                DocumentAction::ReadContent,
            )
            .unwrap();
        f.store
            .record_access(
                actor,
                f.db.case,
                &loaded,
                DocumentAction::ReadContent,
                f.db.at,
            )
            .unwrap();
        f.db.admin
            .execute(
                "DELETE FROM case_memberships WHERE case_id=$1 AND user_id=$2",
                &[&f.db.case.as_uuid(), &actor.as_uuid()],
            )
            .unwrap();
        let before = f.content_events();
        assert!(matches!(
            f.store.record_access(
                actor,
                f.db.case,
                &loaded,
                DocumentAction::ReadContent,
                f.db.at,
            ),
            Err(ApplicationError::DocumentNotFound(_))
        ));
        assert_eq!(f.content_events(), before);
    }
    let client = f.db.user("client", true);
    assert!(matches!(
        f.store.load(
            client,
            f.db.case,
            f.record.id,
            VersionSelection::Exact(f.record.version),
            DocumentAction::ReadContent,
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    let before = f.content_events();
    let original = f.record.clone();
    let mut altered = original.vault.clone();
    let last = altered.len() - 1;
    altered[last] ^= 1;
    f.db.admin
        .batch_execute("ALTER TABLE documents DISABLE TRIGGER USER")
        .unwrap();
    f.db.admin
        .execute(
            "UPDATE documents SET vault=$1 WHERE id=$2 AND version=$3",
            &[
                &altered,
                &original.id.as_uuid(),
                &i64::from(original.version.get()),
            ],
        )
        .unwrap();
    f.db.admin
        .batch_execute("ALTER TABLE documents ENABLE TRIGGER USER")
        .unwrap();
    assert!(matches!(
        f.store.record_access(
            f.db.owner,
            f.db.case,
            &original,
            DocumentAction::ReadContent,
            f.db.at,
        ),
        Err(ApplicationError::DocumentContentValidationFailed(
            DocumentIntegrityFailure::SnapshotChanged
        ))
    ));
    assert_eq!(f.content_events(), before);
}

#[test]
fn content_size_preflight_rejects_large_legacy_vaults_after_authorization() {
    let Some(mut f) = Fixture::new() else { return };
    let mut legacy = record(
        DocumentId::new(),
        DocumentVersion::initial(),
        b"large legacy",
    );
    legacy.vault.resize(16 * 1024 * 1024 + 98, 0);
    f.store
        .insert(f.db.owner, f.db.case, legacy.clone(), f.db.at)
        .unwrap();
    let outsider = f.db.user("litigator", false);
    assert!(matches!(
        f.store.load(
            outsider,
            f.db.case,
            legacy.id,
            VersionSelection::Exact(legacy.version),
            DocumentAction::ReadContent,
        ),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        f.store.load(
            f.db.owner,
            f.db.case,
            legacy.id,
            VersionSelection::Exact(legacy.version),
            DocumentAction::ReadContent,
        ),
        Err(ApplicationError::DocumentContentTooLarge)
    ));
    assert_eq!(f.content_events(), 0);
    assert_eq!(
        f.store
            .load(
                f.db.owner,
                f.db.case,
                legacy.id,
                VersionSelection::Exact(legacy.version),
                DocumentAction::Verify,
            )
            .unwrap(),
        legacy
    );
}
