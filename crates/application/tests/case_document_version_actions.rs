mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;

use application::documents::{
    CaseDocumentWorkflow, DocumentAction, DocumentVersionRef, VersionSelection,
};
use application::ApplicationError;
use case_document_support::{identity, service, MockStore};
use domain::cases::CaseId;
use domain::crypto::{DocumentId, DocumentVersion};
use domain::identity::{Permission, Role};

#[test]
fn version_actions_have_explicit_permissions_and_stable_audit_names() {
    assert_eq!(
        DocumentAction::Append.permission(),
        Permission::AppendDocument
    );
    assert_eq!(
        DocumentAction::Append.audit_action(),
        "document.version_added"
    );
    assert_eq!(
        DocumentAction::History.permission(),
        Permission::ReadDocument
    );
    assert_eq!(
        DocumentAction::History.audit_action(),
        "document.versions_listed"
    );
}

#[test]
fn implicit_actions_reject_ambiguous_documents_before_preparing_or_committing() {
    let (identity, _) = identity(Role::Owner, 3);
    let mut store = MockStore::new();
    store
        .expect_load()
        .times(3)
        .withf(|_, _, _, selection, _| *selection == VersionSelection::Only)
        .returning(|_, _, _, _, _| Err(ApplicationError::DocumentVersionRequired));
    let service = service(store, identity);
    let case = CaseId::new();
    let id = DocumentId::new();
    assert!(matches!(
        service.seal("session", case, id),
        Err(ApplicationError::DocumentVersionRequired)
    ));
    assert!(matches!(
        service.verify("session", case, id),
        Err(ApplicationError::DocumentVersionRequired)
    ));
    assert!(matches!(
        service.export_evidence("session", case, id),
        Err(ApplicationError::DocumentVersionRequired)
    ));
}

#[test]
fn explicit_and_implicit_actions_commit_the_snapshot_resolved_before_preparation() {
    for explicit in [false, true] {
        for action in [
            DocumentAction::Seal,
            DocumentAction::Verify,
            DocumentAction::Export,
        ] {
            let (identity, actor) = identity(Role::Litigator, 2);
            let case = CaseId::new();
            let processor = crypto::processor();
            let pending = processor
                .prepare_version(
                    DocumentId::new(),
                    DocumentVersion::new(7).unwrap(),
                    "original.txt",
                    b"original bytes",
                )
                .unwrap();
            let record = if action == DocumentAction::Seal {
                pending
            } else {
                processor.seal(&pending).unwrap()
            };
            let reference = DocumentVersionRef {
                id: record.id,
                version: record.version,
            };
            let expected = record.clone();
            let selection = if explicit {
                VersionSelection::Exact(reference.version)
            } else {
                VersionSelection::Only
            };
            let mut store = MockStore::new();
            store
                .expect_load()
                .times(1)
                .withf(move |user, scope, id, requested, operation| {
                    *user == actor
                        && *scope == case
                        && *id == reference.id
                        && *requested == selection
                        && *operation == action
                })
                .return_once(move |_, _, _, _, _| Ok(record));
            if action == DocumentAction::Seal {
                store
                    .expect_seal()
                    .times(1)
                    .withf(move |user, scope, record, _| {
                        *user == actor
                            && *scope == case
                            && record.id == reference.id
                            && record.version == reference.version
                            && record.vault == expected.vault
                            && record.is_sealed()
                    })
                    .returning(|_, _, _, _| Ok(()));
            } else {
                store
                    .expect_record_access()
                    .times(1)
                    .withf(move |user, scope, record, operation, _| {
                        *user == actor
                            && *scope == case
                            && *record == expected
                            && *operation == action
                    })
                    .returning(|_, _, _, _, _| Ok(()));
            }
            let service = service(store, identity);
            match (action, explicit) {
                (DocumentAction::Seal, true) => {
                    assert_eq!(
                        service
                            .seal_version("session", case, reference)
                            .unwrap()
                            .document
                            .version,
                        reference.version
                    );
                }
                (DocumentAction::Seal, false) => {
                    assert_eq!(
                        service
                            .seal("session", case, reference.id)
                            .unwrap()
                            .document
                            .version,
                        reference.version
                    );
                }
                (DocumentAction::Verify, true) => {
                    service.verify_version("session", case, reference).unwrap();
                }
                (DocumentAction::Verify, false) => {
                    service.verify("session", case, reference.id).unwrap();
                }
                (DocumentAction::Export, true) => {
                    service.export_version("session", case, reference).unwrap();
                }
                _ => {
                    service
                        .export_evidence("session", case, reference.id)
                        .unwrap();
                }
            }
        }
    }
}

#[test]
fn exact_history_reads_propagate_audit_failure_and_hidden_resource_errors() {
    let (identity, _) = identity(Role::Owner, 2);
    let mut store = MockStore::new();
    store
        .expect_history()
        .times(1)
        .returning(|_, _, _, _, _| Err(ApplicationError::Port("audit write failed".into())));
    store
        .expect_get()
        .times(1)
        .returning(|_, _, id, _, _| Err(ApplicationError::DocumentNotFound(id.to_string())));
    let service = service(store, identity);
    let reference = DocumentVersionRef {
        id: DocumentId::new(),
        version: DocumentVersion::initial(),
    };
    let case = CaseId::new();
    assert!(matches!(
        service.history(
            "session",
            case,
            reference.id,
            application::documents::VersionQuery::new(1, None).unwrap()
        ),
        Err(ApplicationError::Port(_))
    ));
    assert!(matches!(
        service.get_version("session", case, reference),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}
