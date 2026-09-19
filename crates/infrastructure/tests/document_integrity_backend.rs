mod document_content_support;

use application::document_integrity::{
    DocumentIntegrityFailure, DocumentIntegrityQuery, DocumentIntegrityStore,
};
use application::ApplicationError;
use document_content_support::Fixture;

#[test]
fn rejected_content_is_durable_replayable_and_only_visible_to_current_owners() {
    let Some(mut f) = Fixture::new() else { return };
    let mut observation = f.observation();
    let receipt = f.store.record_rejection(&observation).unwrap();
    assert_eq!(receipt.observation_id, observation.observation_id);
    assert_eq!(f.store.record_rejection(&observation).unwrap(), receipt);
    let incident = f
        .store
        .get(f.db.owner, receipt.incident_id, f.db.at)
        .unwrap();
    assert_eq!(incident.id, receipt.incident_id);
    assert_eq!(incident.reference.id, observation.record.id);
    assert_eq!(incident.reference.version, observation.record.version);
    assert_eq!(incident.failure, DocumentIntegrityFailure::DigestMismatch);
    assert_eq!(incident.expected_digest, observation.record.digest);
    assert_eq!(incident.requester, observation.requester);
    let page = f
        .store
        .list(
            f.db.owner,
            DocumentIntegrityQuery::new(1, None).unwrap(),
            f.db.at,
        )
        .unwrap();
    assert_eq!(page.incidents, vec![incident]);
    assert!(!page.has_more);
    assert!(page.next_after_id.is_none());
    for role in ["litigator", "paralegal", "client"] {
        let actor = f.db.user(role, true);
        assert!(matches!(
            f.store.get(actor, receipt.incident_id, f.db.at),
            Err(ApplicationError::PermissionDenied)
        ));
    }
    let other_owner = f.db.user("owner", false);
    f.store
        .get(other_owner, receipt.incident_id, f.db.at)
        .unwrap();
    f.db.admin
        .execute(
            "UPDATE users SET role='litigator' WHERE id=$1",
            &[&other_owner.as_uuid()],
        )
        .unwrap();
    assert!(matches!(
        f.store.get(other_owner, receipt.incident_id, f.db.at),
        Err(ApplicationError::PermissionDenied)
    ));
    observation.failure = DocumentIntegrityFailure::AuthenticationFailed;
    assert!(matches!(
        f.store.record_rejection(&observation),
        Err(ApplicationError::DocumentIntegrityObservationConflict)
    ));
    assert_eq!(f.content_events(), 0);
    let count: i64 =
        f.db.admin
            .query_one("SELECT count(*) FROM document_integrity_incidents", &[])
            .unwrap()
            .get(0);
    assert_eq!(count, 1);
}

#[test]
fn rejected_content_commits_no_partial_incident_when_its_audit_fails() {
    let Some(mut f) = Fixture::new() else { return };
    let first = f.observation();
    f.store.record_rejection(&first).unwrap();
    let before: i64 =
        f.db.admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get(0);
    f.db.admin.batch_execute(
        "ALTER TABLE audit_events ADD CONSTRAINT reject_document_failure CHECK(action <> 'document.content_rejected') NOT VALID",
    ).unwrap();
    let second = f.observation();
    assert!(f.store.record_rejection(&second).is_err());
    f.db.admin
        .batch_execute("ALTER TABLE audit_events DROP CONSTRAINT reject_document_failure")
        .unwrap();
    let count: i64 =
        f.db.admin
            .query_one("SELECT count(*) FROM document_integrity_incidents", &[])
            .unwrap()
            .get(0);
    assert_eq!(count, 1);
    let after: i64 =
        f.db.admin
            .query_one("SELECT count(*) FROM audit_events", &[])
            .unwrap()
            .get(0);
    assert_eq!(after, before);
    assert_eq!(f.content_events(), 0);
}
