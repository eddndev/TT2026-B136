mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{
    CaseDocumentWorkflow, CurrentDocumentMetadata, DocumentMetadata, DocumentMetadataRevision,
    MetadataActorSnapshot, MetadataPage, MetadataQuery, MetadataRevision,
};
use application::ApplicationError;
use case_document_support::{identity, service, MockIdentity, MockStore};
use domain::cases::CaseId;
use domain::clock::Clock;
use domain::crypto::{DocumentId, Sha256Digest};
use domain::identity::{Role, UserId};

fn query() -> MetadataQuery {
    MetadataQuery::new(2, Some(8)).unwrap()
}

fn values() -> DocumentMetadata {
    DocumentMetadata::new(Some("Escrito"), None, &["Urgente".into()]).unwrap()
}

#[test]
fn metadata_operations_reject_revoked_sessions_and_clients_before_storage() {
    for revoked in [true, false] {
        let credentials = if revoked {
            let mut credentials = MockIdentity::new();
            credentials
                .expect_authenticate()
                .times(3)
                .returning(|_| Err(ApplicationError::InvalidSession));
            credentials
        } else {
            identity(Role::Client, 3).0
        };
        let service = service(MockStore::new(), credentials);
        let case = CaseId::new();
        let id = DocumentId::new();
        for error in [
            service.get_metadata("session", case, id).unwrap_err(),
            service
                .replace_metadata("session", case, id, MetadataRevision::new(0), values())
                .unwrap_err(),
            service
                .metadata_history("session", case, id, query())
                .unwrap_err(),
        ] {
            assert!(if revoked {
                matches!(error, ApplicationError::InvalidSession)
            } else {
                matches!(error, ApplicationError::PermissionDenied)
            });
        }
    }
}

#[test]
fn metadata_reads_forward_scope_cursor_and_preserve_historical_actor_snapshot() {
    let (credentials, actor) = identity(Role::Paralegal, 2);
    let case = CaseId::new();
    let id = DocumentId::new();
    let expected = CurrentDocumentMetadata {
        metadata_revision: MetadataRevision::new(7),
        values: values(),
    };
    let row = DocumentMetadataRevision {
        metadata_revision: MetadataRevision::new(6),
        values: values(),
        metadata_digest: Sha256Digest::from_array([3; 32]),
        changed_at: crypto::TestClock.now(),
        changed_by: MetadataActorSnapshot {
            id: UserId::new(),
            email: "historical@example.com".into(),
        },
    };
    let page = MetadataPage {
        revisions: vec![row],
        has_more: true,
        next_before_revision: Some(MetadataRevision::new(6)),
    };
    let returned = expected.clone();
    let returned_page = page.clone();
    let mut store = MockStore::new();
    store
        .expect_get_metadata()
        .times(1)
        .withf(move |user, scope, document, at| {
            *user == actor && *scope == case && *document == id && *at == crypto::TestClock.now()
        })
        .return_once(move |_, _, _, _| Ok(returned));
    store
        .expect_metadata_history()
        .times(1)
        .withf(move |user, scope, document, requested, at| {
            *user == actor
                && *scope == case
                && *document == id
                && *requested == query()
                && *at == crypto::TestClock.now()
        })
        .return_once(move |_, _, _, _, _| Ok(returned_page));
    let service = service(store, credentials);
    assert_eq!(service.get_metadata("session", case, id).unwrap(), expected);
    assert_eq!(
        service
            .metadata_history("session", case, id, query())
            .unwrap(),
        page
    );
}

#[test]
fn replacement_passes_expected_revision_and_returns_commit_projection() {
    let (credentials, actor) = identity(Role::Litigator, 1);
    let case = CaseId::new();
    let id = DocumentId::new();
    let expected = CurrentDocumentMetadata {
        metadata_revision: MetadataRevision::new(8),
        values: values(),
    };
    let returned = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_replace_metadata()
        .times(1)
        .withf(move |user, scope, document, revision, metadata, at| {
            *user == actor
                && *scope == case
                && *document == id
                && *revision == MetadataRevision::new(7)
                && *metadata == values()
                && *at == crypto::TestClock.now()
        })
        .return_once(move |_, _, _, _, _, _| Ok(returned));
    assert_eq!(
        service(store, credentials)
            .replace_metadata("session", case, id, MetadataRevision::new(7), values())
            .unwrap(),
        expected
    );
}

#[test]
fn metadata_port_failures_never_return_values() {
    let (credentials, _) = identity(Role::Owner, 6);
    let mut store = MockStore::new();
    store
        .expect_get_metadata()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::DocumentNotFound("missing".into())));
    store
        .expect_metadata_history()
        .times(1)
        .returning(|_, _, _, _, _| Err(ApplicationError::Port("audit failed".into())));
    let mut sequence = mockall::Sequence::new();
    for error in [
        ApplicationError::DocumentMetadataConflict,
        ApplicationError::DocumentMetadataRevisionExhausted,
        ApplicationError::StoredDocumentMetadataInconsistent("digest mismatch".into()),
        ApplicationError::PermissionDenied,
    ] {
        store
            .expect_replace_metadata()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_, _, _, _, _, _| Err(error));
    }
    let service = service(store, credentials);
    let case = CaseId::new();
    let id = DocumentId::new();
    assert!(matches!(
        service.get_metadata("session", case, id),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        service.metadata_history("session", case, id, query()),
        Err(ApplicationError::Port(_))
    ));
    let errors: Vec<_> = (0..4)
        .map(|_| {
            service
                .replace_metadata("session", case, id, MetadataRevision::new(7), values())
                .unwrap_err()
        })
        .collect();
    assert!(matches!(
        errors[0],
        ApplicationError::DocumentMetadataConflict
    ));
    assert!(matches!(
        errors[1],
        ApplicationError::DocumentMetadataRevisionExhausted
    ));
    assert!(matches!(
        errors[2],
        ApplicationError::StoredDocumentMetadataInconsistent(_)
    ));
    assert!(matches!(errors[3], ApplicationError::PermissionDenied));
}
