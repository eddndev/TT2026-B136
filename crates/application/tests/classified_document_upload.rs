mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, CurrentDocumentMetadata, DocumentAction,
    DocumentMetadata, DocumentOverview, DocumentSummary, MetadataRevision,
};
use application::identity::Principal;
use application::ApplicationError;
use case_document_support::{identity, service, MockIdentity, MockStore};
use domain::cases::CaseId;
use domain::clock::Clock;
use domain::identity::{Role, UserId};

fn check_both(store: &mut MockStore, actor: UserId, case: CaseId) {
    for action in [DocumentAction::Upload, DocumentAction::Classify] {
        store
            .expect_check_access()
            .times(1)
            .withf(move |user, scope, requested| {
                *user == actor && *scope == case && *requested == action
            })
            .returning(|_, _, _| Ok(()));
    }
}

#[test]
fn classified_upload_checks_both_permissions_and_returns_the_atomic_commit_overview() {
    for values in [
        DocumentMetadata::empty(),
        DocumentMetadata::new(Some("Escrito"), None, &["Urgente".into()]).unwrap(),
    ] {
        let (credentials, actor) = identity(Role::Paralegal, 2);
        let case = CaseId::new();
        let expected_values = values.clone();
        let returned_values = values.clone();
        let mut store = MockStore::new();
        check_both(&mut store, actor, case);
        store
            .expect_insert_with_metadata()
            .times(1)
            .withf(move |user, scope, record, metadata, at| {
                *user == actor
                    && *scope == case
                    && *metadata == expected_values
                    && record.version.get() == 1
                    && record.name == "a.txt"
                    && record.vault.starts_with(b"DVLT1")
                    && !record.is_sealed()
                    && *at == crypto::TestClock.now()
            })
            .return_once(move |_, scope, record, _, _| {
                Ok(DocumentOverview {
                    content: CaseDocumentSummary {
                        case_id: scope,
                        document: DocumentSummary::from(&record),
                    },
                    current_metadata: CurrentDocumentMetadata {
                        metadata_revision: MetadataRevision::new(1),
                        values: returned_values,
                    },
                })
            });
        let result = service(store, credentials)
            .upload_with_metadata("session", case, "a.txt", b"data", values.clone())
            .unwrap();
        assert_eq!(result.content.case_id, case);
        assert_eq!(result.current_metadata.metadata_revision.get(), 1);
        assert_eq!(result.current_metadata.values, values);
    }
}

#[test]
fn classified_upload_rejects_client_or_revoked_session_without_storage() {
    let (credentials, _) = identity(Role::Client, 1);
    assert!(matches!(
        service(MockStore::new(), credentials).upload_with_metadata(
            "session",
            CaseId::new(),
            "a.txt",
            b"data",
            DocumentMetadata::empty()
        ),
        Err(ApplicationError::PermissionDenied)
    ));
    let mut credentials = MockIdentity::new();
    credentials
        .expect_authenticate()
        .times(1)
        .returning(|_| Err(ApplicationError::InvalidSession));
    assert!(matches!(
        service(MockStore::new(), credentials).upload_with_metadata(
            "session",
            CaseId::new(),
            "a.txt",
            b"data",
            DocumentMetadata::empty()
        ),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn rejected_classification_scope_stops_before_prepare_and_commit() {
    let (credentials, actor) = identity(Role::Owner, 1);
    let case = CaseId::new();
    let mut store = MockStore::new();
    store
        .expect_check_access()
        .withf(|_, _, action| *action == DocumentAction::Upload)
        .returning(|_, _, _| Ok(()));
    store
        .expect_check_access()
        .times(1)
        .withf(move |user, scope, action| {
            *user == actor && *scope == case && *action == DocumentAction::Classify
        })
        .returning(|_, _, _| Err(ApplicationError::CaseNotFound));
    assert!(matches!(
        service(store, credentials).upload_with_metadata(
            "session",
            case,
            "invalid/name",
            b"data",
            DocumentMetadata::empty()
        ),
        Err(ApplicationError::CaseNotFound)
    ));
}

#[test]
fn invalid_content_preparation_never_commits_a_classification() {
    let (credentials, actor) = identity(Role::Owner, 1);
    let case = CaseId::new();
    let mut store = MockStore::new();
    check_both(&mut store, actor, case);
    assert!(service(store, credentials)
        .upload_with_metadata(
            "session",
            case,
            "invalid/name",
            b"data",
            DocumentMetadata::empty()
        )
        .is_err());
}

#[test]
fn classified_upload_reauthenticates_the_same_actor_and_role_before_commit() {
    for outcome in 0..3 {
        let actor = UserId::new();
        let mut credentials = MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        credentials
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| {
                Ok(Principal {
                    id: actor,
                    email: "actor@example.com".into(),
                    role: Role::Owner,
                })
            });
        credentials
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |_| match outcome {
                0 => Err(ApplicationError::InvalidSession),
                1 => Ok(Principal {
                    id: UserId::new(),
                    email: "other@example.com".into(),
                    role: Role::Owner,
                }),
                _ => Ok(Principal {
                    id: actor,
                    email: "actor@example.com".into(),
                    role: Role::Client,
                }),
            });
        let case = CaseId::new();
        let mut store = MockStore::new();
        check_both(&mut store, actor, case);
        let error = service(store, credentials)
            .upload_with_metadata("session", case, "a.txt", b"data", DocumentMetadata::empty())
            .unwrap_err();
        assert!(if outcome == 2 {
            matches!(error, ApplicationError::PermissionDenied)
        } else {
            matches!(error, ApplicationError::InvalidSession)
        });
    }
}

#[test]
fn failed_classified_commit_never_falls_back_to_separate_mutations_or_reads() {
    let (credentials, actor) = identity(Role::Litigator, 2);
    let case = CaseId::new();
    let mut store = MockStore::new();
    check_both(&mut store, actor, case);
    store
        .expect_insert_with_metadata()
        .times(1)
        .returning(|_, _, _, _, _| Err(ApplicationError::Port("second audit event failed".into())));
    assert!(matches!(
        service(store, credentials).upload_with_metadata(
            "session",
            case,
            "a.txt",
            b"data",
            DocumentMetadata::empty()
        ),
        Err(ApplicationError::Port(_))
    ));
}
