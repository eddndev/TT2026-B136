#[allow(dead_code)]
mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod document_content_support;
#[allow(dead_code)]
mod document_format_support;

use application::{
    document_content::DocumentContentWorkflow,
    documents::{DocumentAction, VersionSelection},
    ApplicationError,
};
use case_document_support::MockStore;
use document_content_support::{identity, principal, reference, service, MockIncidents};
use domain::{
    cases::CaseId,
    crypto::{document_aad, DocumentVersion},
    identity::{Permission, Role, UserId},
};

#[test]
fn pending_historical_bytes_are_released_only_after_exact_access_is_committed() {
    let bytes = b"old unsealed\0\xff bytes";
    let writer = crypto::processor();
    let old = writer.prepare("old.bin", bytes).unwrap();
    let newer = writer
        .prepare_version(old.id, DocumentVersion::new(2).unwrap(), "new.bin", b"new")
        .unwrap();
    let requested = reference(&old);
    let case = CaseId::new();
    let principal = principal();
    let actor = principal.id;
    let mut store = MockStore::new();
    let mut sequence = mockall::Sequence::new();
    let loaded = old.clone();
    store
        .expect_load()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(move |who, found_case, id, selection, action| {
            *who == actor
                && *found_case == case
                && *id == requested.id
                && *selection == VersionSelection::Exact(requested.version)
                && *action == DocumentAction::ReadContent
        })
        .returning(move |_, _, _, _, _| Ok(loaded.clone()));
    let committed = old.clone();
    store
        .expect_record_access()
        .times(1)
        .in_sequence(&mut sequence)
        .withf(move |who, found_case, record, action, _| {
            *who == actor
                && *found_case == case
                && record == &committed
                && *action == DocumentAction::ReadContent
        })
        .returning(|_, _, _, _, _| Ok(()));
    let (reader, observed) = document_format_support::processor();
    let result = service(store, identity(principal, 2), reader, MockIncidents::new())
        .content_version("session", case, requested)
        .unwrap();
    assert_eq!(result.case_id, case);
    assert_eq!(result.reference, requested);
    assert_eq!(result.file_name, old.name);
    assert_eq!(result.digest, old.digest);
    assert_eq!(result.bytes.as_slice(), bytes);
    assert_ne!(result.reference.version, newer.version);
    let observed = observed.lock().unwrap();
    assert_eq!(observed.events, ["unwrap", "open", "hash"]);
    assert_eq!(observed.aad, [document_aad(old.id, old.version).to_vec()]);
    assert_eq!(
        DocumentAction::ReadContent.permission(),
        Permission::ReadDocument
    );
    assert_eq!(
        DocumentAction::ReadContent.audit_action(),
        "document.content_authorized"
    );
}

#[test]
fn content_does_not_evaluate_captured_signature_or_timestamp() {
    let writer = crypto::processor();
    let mut record = writer
        .seal(&writer.prepare("signed.bin", b"original").unwrap())
        .unwrap();
    record.evidence.as_mut().unwrap().signature = vec![0];
    record.evidence.as_mut().unwrap().timestamp_token = vec![0];
    let requested = reference(&record);
    let case = CaseId::new();
    let mut store = MockStore::new();
    store
        .expect_load()
        .times(1)
        .returning(move |_, _, _, _, _| Ok(record.clone()));
    store
        .expect_record_access()
        .times(1)
        .returning(|_, _, _, _, _| Ok(()));
    let (reader, observed) = document_format_support::processor();
    let result = service(
        store,
        identity(principal(), 2),
        reader,
        MockIncidents::new(),
    )
    .content_version("session", case, requested)
    .unwrap();
    assert_eq!(result.bytes.as_slice(), b"original");
    assert_eq!(observed.lock().unwrap().events, ["unwrap", "open", "hash"]);
}

#[test]
fn client_is_denied_before_loading_or_observing_document_bytes() {
    let record = crypto::processor().prepare("file.bin", b"private").unwrap();
    let mut principal = principal();
    principal.role = Role::Client;
    let result = service(
        MockStore::new(),
        identity(principal, 1),
        crypto::processor(),
        MockIncidents::new(),
    )
    .content_version("session", CaseId::new(), reference(&record));
    assert!(matches!(result, Err(ApplicationError::PermissionDenied)));
}

#[test]
fn reauthentication_requires_the_complete_unchanged_principal() {
    for changed_field in 0..4 {
        let record = crypto::processor().prepare("file.bin", b"private").unwrap();
        let requested = reference(&record);
        let principal = principal();
        let first = principal.clone();
        let mut after = principal;
        match changed_field {
            0 => after.id = UserId::new(),
            1 => after.email = "renamed@example.test".into(),
            2 => after.role = Role::Owner,
            _ => {}
        }
        let mut identity = case_document_support::MockIdentity::new();
        let mut sequence = mockall::Sequence::new();
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| Ok(first.clone()));
        identity
            .expect_authenticate()
            .times(1)
            .in_sequence(&mut sequence)
            .returning(move |_| {
                if changed_field == 3 {
                    Err(ApplicationError::InvalidSession)
                } else {
                    Ok(after.clone())
                }
            });
        let mut store = MockStore::new();
        store
            .expect_load()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(record.clone()));
        let result = service(store, identity, crypto::processor(), MockIncidents::new())
            .content_version("session", CaseId::new(), requested);
        assert!(
            matches!(result, Err(ApplicationError::InvalidSession)),
            "field {changed_field}"
        );
    }
}

#[test]
fn a_store_response_for_another_identity_or_version_is_never_delivered() {
    for changed_id in [true, false] {
        let mut record = crypto::processor().prepare("file.bin", b"private").unwrap();
        let requested = reference(&record);
        if changed_id {
            record.id = domain::crypto::DocumentId::new();
        } else {
            record.version = DocumentVersion::new(2).unwrap();
        }
        let mut store = MockStore::new();
        store
            .expect_load()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(record.clone()));
        let (reader, observed) = document_format_support::processor();
        let result = service(
            store,
            identity(principal(), 1),
            reader,
            MockIncidents::new(),
        )
        .content_version("session", CaseId::new(), requested);
        assert!(matches!(
            result,
            Err(ApplicationError::StoredDocumentInconsistent(_))
        ));
        assert!(observed.lock().unwrap().events.is_empty());
    }
}
