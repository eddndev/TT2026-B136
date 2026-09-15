use application::documents::DocumentAction;
use domain::identity::{Permission, Role};

#[test]
fn document_actions_preserve_the_existing_permission_matrix() {
    for (action, permission, event) in [
        (
            DocumentAction::Upload,
            Permission::CreateDocument,
            "document.uploaded",
        ),
        (
            DocumentAction::Seal,
            Permission::SealDocument,
            "document.sealed",
        ),
        (
            DocumentAction::Verify,
            Permission::VerifyDocument,
            "document.verified",
        ),
        (
            DocumentAction::Export,
            Permission::ExportEvidence,
            "document.evidence_exported",
        ),
    ] {
        assert_eq!(action.permission(), permission);
        assert_eq!(action.audit_action(), event);
        assert!(!Role::Client.allows(action.permission()));
    }
}

mod case_document_support;
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{CaseDocumentWorkflow, DocumentWorkflow};
use application::ApplicationError;
use case_document_support::{identity, service, MockIdentity, MockStore};
use domain::cases::CaseId;
use domain::crypto::DocumentId;

#[test]
fn invalid_sessions_never_reach_document_storage() {
    let mut identity = MockIdentity::new();
    identity
        .expect_authenticate()
        .times(5)
        .returning(|_| Err(ApplicationError::InvalidSession));
    let service = service(MockStore::new(), identity);
    let case = CaseId::new();
    let document = DocumentId::new();
    assert!(matches!(
        service.upload("revoked", case, "a.txt", b"data"),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.seal("revoked", case, document),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.verify("revoked", case, document),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.export_evidence("revoked", case, document),
        Err(ApplicationError::InvalidSession)
    ));
    assert!(matches!(
        service.verify_audit("revoked"),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn client_permissions_and_paralegal_sealing_remain_denied_before_storage() {
    let (identity, _) = identity(Role::Client, 5);
    let service = service(MockStore::new(), identity);
    let case = CaseId::new();
    let document = DocumentId::new();
    assert!(matches!(
        service.upload("session", case, "a.txt", b"data"),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        service.seal("session", case, document),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        service.verify("session", case, document),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        service.export_evidence("session", case, document),
        Err(ApplicationError::PermissionDenied)
    ));
    assert!(matches!(
        service.verify_audit("session"),
        Err(ApplicationError::PermissionDenied)
    ));
    let (identity, _) = case_document_support::identity(Role::Paralegal, 1);
    let service = case_document_support::service(MockStore::new(), identity);
    assert!(matches!(
        service.seal("session", case, document),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn authorized_upload_preserves_local_crypto_format_and_commits_authenticated_scope() {
    let (identity, actor) = identity(Role::Litigator, 2);
    let case = CaseId::new();
    let local = crypto::workflow()
        .upload("offline", "a.txt", b"data")
        .unwrap();
    let mut store = MockStore::new();
    store
        .expect_check_access()
        .times(1)
        .withf(move |user, scope, action| {
            *user == actor && *scope == case && *action == DocumentAction::Upload
        })
        .returning(|_, _, _| Ok(()));
    store
        .expect_insert()
        .times(1)
        .withf(move |user, scope, record, _| {
            *user == actor
                && *scope == case
                && record.vault.starts_with(b"DVLT1")
                && !record.is_sealed()
        })
        .returning(|_, _, _, _| Ok(()));
    let result = service(store, identity)
        .upload("session", case, "a.txt", b"data")
        .unwrap();
    assert_eq!(result.case_id, case);
    assert_eq!(result.document.digest_hex, local.digest_hex);
    assert_eq!(result.document.version, local.version);
}

#[test]
fn a_rejected_preparation_scope_never_persists_an_upload() {
    let (identity, _) = identity(Role::Owner, 1);
    let mut store = MockStore::new();
    store
        .expect_check_access()
        .returning(|_, _, _| Err(ApplicationError::CaseNotFound));
    assert!(matches!(
        service(store, identity).upload("session", CaseId::new(), "a.txt", b"data"),
        Err(ApplicationError::CaseNotFound)
    ));
}

#[test]
fn seal_commit_rejection_never_returns_prepared_evidence_as_success() {
    let (identity, _) = identity(Role::Litigator, 2);
    let pending = crypto::processor().prepare("a.txt", b"data").unwrap();
    let id = pending.id;
    let expected = pending.clone();
    let mut store = MockStore::new();
    store
        .expect_load()
        .return_once(move |_, _, _, _| Ok(pending));
    store
        .expect_seal()
        .times(1)
        .withf(move |_, _, record, _| {
            record.vault == expected.vault
                && record.id == expected.id
                && record.version == expected.version
                && record.is_sealed()
        })
        .returning(|_, _, _, _| Err(ApplicationError::PermissionDenied));
    assert!(matches!(
        service(store, identity).seal("session", CaseId::new(), id),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn verification_and_export_recheck_storage_before_releasing_results() {
    for action in [DocumentAction::Verify, DocumentAction::Export] {
        let (identity, _) = identity(Role::Paralegal, 2);
        let processor = crypto::processor();
        let record = processor
            .seal(&processor.prepare("a.txt", b"data").unwrap())
            .unwrap();
        let id = record.id;
        let mut store = MockStore::new();
        store
            .expect_load()
            .withf(move |_, _, _, requested| *requested == action)
            .return_once(move |_, _, _, _| Ok(record));
        store
            .expect_record_access()
            .times(1)
            .withf(move |_, _, _, requested, _| *requested == action)
            .returning(|_, _, _, _, _| Err(ApplicationError::PermissionDenied));
        let service = service(store, identity);
        let case = CaseId::new();
        let rejected = match action {
            DocumentAction::Verify => service.verify("session", case, id).err(),
            _ => service.export_evidence("session", case, id).err(),
        };
        assert!(matches!(rejected, Some(ApplicationError::PermissionDenied)));
    }
}

#[test]
fn session_revocation_during_preparation_prevents_every_commit() {
    for action in [
        DocumentAction::Upload,
        DocumentAction::Seal,
        DocumentAction::Verify,
        DocumentAction::Export,
    ] {
        let actor = application::identity::Principal {
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
            .return_once(move |_| Ok(actor));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(|_| Err(ApplicationError::InvalidSession));
        let processor = crypto::processor();
        let pending = processor.prepare("a.txt", b"data").unwrap();
        let record = if action == DocumentAction::Seal {
            pending
        } else {
            processor.seal(&pending).unwrap()
        };
        let id = record.id;
        let mut store = MockStore::new();
        if action == DocumentAction::Upload {
            store.expect_check_access().returning(|_, _, _| Ok(()));
        } else {
            store
                .expect_load()
                .return_once(move |_, _, _, _| Ok(record));
        }
        let service = service(store, identity);
        let case = CaseId::new();
        let error = match action {
            DocumentAction::Upload => service.upload("session", case, "a.txt", b"data").err(),
            DocumentAction::Seal => service.seal("session", case, id).err(),
            DocumentAction::Verify => service.verify("session", case, id).err(),
            DocumentAction::Export => service.export_evidence("session", case, id).err(),
            _ => unreachable!("only prepared document actions are tested"),
        };
        assert!(matches!(error, Some(ApplicationError::InvalidSession)));
    }
}

#[test]
fn a_changed_authenticated_user_cannot_commit_prepared_upload() {
    let mut identity = MockIdentity::new();
    identity.expect_authenticate().times(2).returning(|_| {
        Ok(application::identity::Principal {
            id: domain::identity::UserId::new(),
            email: "actor@example.com".into(),
            role: Role::Owner,
        })
    });
    let mut store = MockStore::new();
    store.expect_check_access().returning(|_, _, _| Ok(()));
    assert!(matches!(
        service(store, identity).upload("session", CaseId::new(), "a.txt", b"data"),
        Err(ApplicationError::InvalidSession)
    ));
}

#[test]
fn wrong_case_errors_from_storage_are_preserved_even_for_owner() {
    let (identity, _) = identity(Role::Owner, 3);
    let mut store = MockStore::new();
    store.expect_load().times(3).returning(|_, _, _, _| {
        Err(ApplicationError::DocumentNotFound(
            "document not found".into(),
        ))
    });
    let service = service(store, identity);
    let case = CaseId::new();
    let document = DocumentId::new();
    assert!(matches!(
        service.seal("session", case, document),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        service.verify("session", case, document),
        Err(ApplicationError::DocumentNotFound(_))
    ));
    assert!(matches!(
        service.export_evidence("session", case, document),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}
