#[allow(dead_code)]
mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod document_content_support;

use application::{document_integrity::*, ApplicationError};
use document_content_support::{identity, principal, MockIncidents};
use domain::{
    cases::CaseId,
    clock::{Clock, OffsetDateTime},
    crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    identity::Role,
};
use std::sync::Arc;

fn incident() -> DocumentIntegrityIncident {
    let at = crypto::TestClock.now();
    DocumentIntegrityIncident {
        id: DocumentIntegrityIncidentId::new(),
        observation_id: DocumentIntegrityObservationId::new(),
        case_id: CaseId::new(),
        reference: DocumentVersionRef {
            id: DocumentId::new(),
            version: DocumentVersion::initial(),
        },
        requester: principal().id,
        failure: DocumentIntegrityFailure::AuthenticationFailed,
        detected_at: at,
        recorded_at: at,
        expected_digest: Sha256Digest::from_array([1; 32]),
        observed_snapshot_digest: Sha256Digest::from_array([2; 32]),
    }
}

#[test]
fn query_limits_are_bounded_and_preserve_the_explicit_cursor() {
    let id = DocumentIntegrityIncidentId::new();
    for limit in [1, 100] {
        let query = DocumentIntegrityQuery::new(limit, Some(id)).unwrap();
        assert_eq!(query.limit(), limit);
        assert_eq!(query.after_id(), Some(id));
    }
    for limit in [0, 101, u32::MAX] {
        assert!(DocumentIntegrityQuery::new(limit, None).is_err());
    }
}

#[test]
fn owner_receives_the_audited_page_after_full_reauthentication() {
    let mut owner = principal();
    owner.role = Role::Owner;
    let actor = owner.id;
    let expected = incident();
    let returned = expected.clone();
    let mut store = MockIncidents::new();
    store
        .expect_list()
        .times(1)
        .withf(move |who, query, _| *who == actor && query.limit() == 1)
        .returning(move |_, _, _| {
            Ok(DocumentIntegrityPage {
                incidents: vec![returned.clone()],
                has_more: true,
                next_after_id: Some(returned.id),
            })
        });
    let service = DocumentIntegrityService::new(
        Arc::new(store),
        Arc::new(identity(owner, 2)),
        Arc::new(crypto::TestClock),
    );
    let page = service
        .list("session", DocumentIntegrityQuery::new(1, None).unwrap())
        .unwrap();
    assert_eq!(page.incidents, [expected.clone()]);
    assert!(page.has_more);
    assert_eq!(page.next_after_id, Some(expected.id));
}

#[test]
fn incident_queries_deny_non_owners_without_accessing_the_store() {
    for role in [Role::Litigator, Role::Paralegal, Role::Client] {
        let mut principal = principal();
        principal.role = role;
        let service = DocumentIntegrityService::new(
            Arc::new(MockIncidents::new()),
            Arc::new(identity(principal, 2)),
            Arc::new(crypto::TestClock),
        );
        assert!(matches!(
            service.list("session", DocumentIntegrityQuery::new(20, None).unwrap()),
            Err(ApplicationError::PermissionDenied)
        ));
        assert!(matches!(
            service.get("session", DocumentIntegrityIncidentId::new()),
            Err(ApplicationError::PermissionDenied)
        ));
    }
}

#[test]
fn incident_detail_rejects_a_different_identity_and_future_observation() {
    for mode in 0..2 {
        let mut record = incident();
        let requested = record.id;
        if mode == 0 {
            record.id = DocumentIntegrityIncidentId::new();
        } else {
            record.detected_at = OffsetDateTime::from_unix_timestamp(2_000_000_000).unwrap();
        }
        let mut owner = principal();
        owner.role = Role::Owner;
        let mut store = MockIncidents::new();
        store
            .expect_get()
            .times(1)
            .returning(move |_, _, _| Ok(record.clone()));
        let service = DocumentIntegrityService::new(
            Arc::new(store),
            Arc::new(identity(owner, 1)),
            Arc::new(crypto::TestClock),
        );
        assert!(matches!(
            service.get("session", requested),
            Err(ApplicationError::StoredDocumentInconsistent(_))
        ));
    }
}
