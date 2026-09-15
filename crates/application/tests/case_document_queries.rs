mod case_document_support;
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, CurrentDocumentMetadata, DocumentAction,
    DocumentMetadata, DocumentMetadataFilter, DocumentOverview, DocumentPage, DocumentQuery,
    DocumentWorkflow, MetadataRevision,
};
use application::ApplicationError;
use case_document_support::{identity, service, MockIdentity, MockStore};
use domain::cases::CaseId;
use domain::crypto::DocumentId;
use domain::identity::{Permission, Role};

fn query() -> DocumentQuery {
    DocumentQuery::new(20, 0, None, None).unwrap()
}

#[test]
fn read_actions_require_metadata_permission_and_distinct_audit_events() {
    for (action, event) in [
        (DocumentAction::List, "document.listed"),
        (DocumentAction::Read, "document.read"),
    ] {
        assert_eq!(action.permission(), Permission::ReadDocument);
        assert_eq!(action.audit_action(), event);
    }
}

#[test]
fn revoked_sessions_and_clients_never_query_metadata_storage() {
    let mut identity_mock = MockIdentity::new();
    identity_mock
        .expect_authenticate()
        .times(2)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let revoked = service(MockStore::new(), identity_mock);
    let (identity_mock, _) = identity(Role::Client, 2);
    let client = service(MockStore::new(), identity_mock);
    let case = CaseId::new();
    let id = DocumentId::new();
    assert!(matches!(
        revoked.list("revoked", case, query()),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        revoked.get("revoked", case, id),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        client.list("session", case, query()),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        client.get("session", case, id),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn reads_forward_authenticated_scope_query_and_audit_time() {
    let (identity_mock, actor) = identity(Role::Paralegal, 2);
    let case = CaseId::new();
    let record = crypto::workflow()
        .upload("offline", "proof.txt", b"proof")
        .unwrap();
    let id = record.id;
    let summary = CaseDocumentSummary {
        case_id: case,
        document: record,
    };
    let summary = DocumentOverview {
        content: summary,
        current_metadata: CurrentDocumentMetadata {
            metadata_revision: MetadataRevision::new(7),
            values: DocumentMetadata::new(Some("Escrito"), Some("Civil"), &["Urgente".into()])
                .unwrap(),
        },
    };
    let result = summary.clone();
    let page = DocumentPage {
        documents: vec![summary.clone()],
        has_more: true,
    };
    let requested = DocumentQuery::new(1, 4, Some("proof"), Some(false))
        .unwrap()
        .with_metadata_filter(
            DocumentMetadataFilter::new(Some("Escrito"), Some("Civil"), Some("Urgente")).unwrap(),
        );
    let expected = requested.clone();
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .withf(move |user, scope, query, at| {
            *user == actor
                && *scope == case
                && *query == expected
                && *at == domain::clock::Clock::now(&crypto::TestClock)
        })
        .return_once(move |_, _, _, _| Ok(page));
    store
        .expect_get_overview()
        .times(1)
        .withf(move |user, scope, requested, at| {
            *user == actor
                && *scope == case
                && *requested == id
                && *at == domain::clock::Clock::now(&crypto::TestClock)
        })
        .return_once(move |_, _, _, _| Ok(summary));
    let service = service(store, identity_mock);
    let listed = service.list("session", case, requested).unwrap();
    assert_eq!(listed.documents, vec![result.clone()]);
    assert!(listed.has_more);
    assert_eq!(service.get("session", case, id).unwrap(), result);
}

#[test]
fn audit_or_authorization_failure_never_returns_metadata() {
    let (identity_mock, _) = identity(Role::Owner, 2);
    let mut store = MockStore::new();
    store
        .expect_list()
        .times(1)
        .returning(|_, _, _, _| Err(ApplicationError::Port("audit write failed".into())));
    store
        .expect_get_overview()
        .times(1)
        .returning(|_, _, id, _| Err(ApplicationError::DocumentNotFound(id.to_string())));
    let service = service(store, identity_mock);
    let case = CaseId::new();
    assert!(matches!(
        service.list("session", case, query()),
        Err(ApplicationError::Port(_))
    ));
    assert!(matches!(
        service.get("session", case, DocumentId::new()),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}
