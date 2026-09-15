mod case_document_support;
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{
    CaseDocumentSummary, CaseDocumentWorkflow, DocumentAction, DocumentSummary, DocumentVersionRef,
    DocumentWorkflow, VersionPage, VersionQuery, VersionSelection,
};
use application::ApplicationError;
use case_document_support::{identity, service, MockIdentity, MockStore};
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};
use domain::identity::Role;

fn version(number: u32) -> DocumentVersion {
    DocumentVersion::new(number).unwrap()
}

#[test]
fn append_preserves_identity_and_uses_expected_successor_after_reauthentication() {
    let (identity, actor) = identity(Role::Paralegal, 2);
    let case = CaseId::new();
    let old = crypto::processor().prepare("old.txt", b"old").unwrap();
    let id = old.id;
    let mut store = MockStore::new();
    store
        .expect_load()
        .times(1)
        .withf(move |user, scope, requested, selection, action| {
            *user == actor
                && *scope == case
                && *requested == id
                && *selection == VersionSelection::Current
                && *action == DocumentAction::Append
        })
        .return_once(move |_, _, _, _, _| Ok(old));
    store
        .expect_append()
        .times(1)
        .withf(move |user, scope, expected, record, _| {
            *user == actor
                && *scope == case
                && *expected == version(1)
                && record.id == id
                && record.version == version(2)
                && record.name == "new.txt"
                && !record.is_sealed()
        })
        .returning(|_, _, _, _, _| Ok(()));
    let result = service(store, identity)
        .append("session", case, id, version(1), "new.txt", b"new")
        .unwrap();
    assert_eq!(result.document.id, id);
    assert_eq!(result.document.version, version(2));
    assert_eq!(result.case_id, case);
    assert_eq!(
        result.document.digest_hex,
        crypto::workflow()
            .upload("offline", "new.txt", b"new")
            .unwrap()
            .digest_hex
    );
}

#[test]
fn stale_or_exhausted_versions_fail_before_encryption_or_reauthentication() {
    for (current, expected) in [(2, 1), (u32::MAX, u32::MAX)] {
        let (identity, _) = identity(Role::Owner, 1);
        let mut record = crypto::processor().prepare("old.txt", b"old").unwrap();
        record.version = version(current);
        let id = record.id;
        let mut store = MockStore::new();
        store
            .expect_load()
            .return_once(move |_, _, _, _, _| Ok(record));
        let result = service(store, identity).append(
            "session",
            CaseId::new(),
            id,
            version(expected),
            "invalid/name",
            b"new",
        );
        if current == u32::MAX {
            assert!(matches!(
                result,
                Err(ApplicationError::DocumentVersionExhausted)
            ));
        } else {
            assert!(matches!(
                result,
                Err(ApplicationError::DocumentVersionConflict)
            ));
        }
    }
}

#[test]
fn revoked_sessions_and_clients_never_reach_any_version_storage() {
    let case = CaseId::new();
    let reference = DocumentVersionRef {
        id: DocumentId::new(),
        version: version(1),
    };
    for client in [false, true] {
        let identity = if client {
            identity(Role::Client, 6).0
        } else {
            let mut mock = MockIdentity::new();
            mock.expect_authenticate()
                .times(6)
                .returning(|_| Err(ApplicationError::InvalidSession));
            mock
        };
        let service = service(MockStore::new(), identity);
        let errors = [
            service
                .append("session", case, reference.id, version(1), "a.txt", b"a")
                .err(),
            service
                .history(
                    "session",
                    case,
                    reference.id,
                    VersionQuery::new(50, None).unwrap(),
                )
                .err(),
            service.get_version("session", case, reference).err(),
            service.seal_version("session", case, reference).err(),
            service.verify_version("session", case, reference).err(),
            service.export_version("session", case, reference).err(),
        ];
        for error in errors {
            if client {
                assert!(matches!(error, Some(ApplicationError::PermissionDenied)));
            } else {
                assert!(matches!(error, Some(ApplicationError::InvalidSession)));
            }
        }
    }
}

#[test]
fn append_does_not_commit_after_session_revocation_during_preparation() {
    let principal = application::identity::Principal {
        id: domain::identity::UserId::new(),
        email: "actor@example.com".into(),
        role: Role::Owner,
    };
    let mut identity = MockIdentity::new();
    let mut sequence = mockall::Sequence::new();
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .return_once(move |_| Ok(principal));
    identity
        .expect_authenticate()
        .times(1)
        .in_sequence(&mut sequence)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let record = crypto::processor().prepare("old.txt", b"old").unwrap();
    let id = record.id;
    let mut store = MockStore::new();
    store
        .expect_load()
        .return_once(move |_, _, _, _, _| Ok(record));
    assert!(matches!(
        service(store, identity).append(
            "session",
            CaseId::new(),
            id,
            version(1),
            "new.txt",
            b"new"
        ),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn append_commit_conflict_or_audit_failure_never_returns_success() {
    for audit_failure in [false, true] {
        let (identity, _) = identity(Role::Owner, 2);
        let record = crypto::processor().prepare("old.txt", b"old").unwrap();
        let id = record.id;
        let mut store = MockStore::new();
        store
            .expect_load()
            .return_once(move |_, _, _, _, _| Ok(record));
        store
            .expect_append()
            .times(1)
            .return_once(move |_, _, _, _, _| {
                Err(if audit_failure {
                    ApplicationError::Port("audit failed".into())
                } else {
                    ApplicationError::DocumentVersionConflict
                })
            });
        assert!(service(store, identity)
            .append("session", CaseId::new(), id, version(1), "new.txt", b"new")
            .is_err());
    }
}

#[test]
fn history_and_version_detail_preserve_exact_query_and_authorized_metadata() {
    let (identity, actor) = identity(Role::Litigator, 2);
    let case = CaseId::new();
    let mut record = crypto::processor().prepare("original.txt", b"old").unwrap();
    record.version = version(7);
    let reference = DocumentVersionRef {
        id: record.id,
        version: record.version,
    };
    let expected = CaseDocumentSummary {
        case_id: case,
        document: DocumentSummary::from(&record),
    };
    let summary = expected.clone();
    let mut store = MockStore::new();
    store
        .expect_history()
        .times(1)
        .withf(move |user, scope, id, query, _| {
            *user == actor
                && *scope == case
                && *id == reference.id
                && query.limit() == 1
                && query.before_version() == Some(version(8))
        })
        .return_once(move |_, _, _, _, _| {
            Ok(VersionPage {
                versions: vec![summary],
                has_more: false,
                next_before_version: None,
                first_available_version: version(7),
            })
        });
    let summary = expected.clone();
    store
        .expect_get()
        .times(1)
        .withf(move |user, scope, id, selection, _| {
            *user == actor
                && *scope == case
                && *id == reference.id
                && *selection == VersionSelection::Exact(reference.version)
        })
        .return_once(move |_, _, _, _, _| Ok(summary));
    let service = service(store, identity);
    let page = service
        .history(
            "session",
            case,
            reference.id,
            VersionQuery::new(1, Some(8)).unwrap(),
        )
        .unwrap();
    assert_eq!(page.versions, vec![expected.clone()]);
    assert_eq!(page.first_available_version, version(7));
    assert_eq!(
        service.get_version("session", case, reference).unwrap(),
        expected
    );
}
