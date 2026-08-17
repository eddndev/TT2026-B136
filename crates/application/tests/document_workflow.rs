#[path = "support/document_workflow.rs"]
mod support;

use application::documents::DocumentWorkflow;
use application::ApplicationError;
use domain::audit::ChainVerification;
use domain::crypto::DocumentId;

#[test]
fn upload_seal_verify_export_and_audit_form_one_workflow() {
    let workflow = support::workflow();

    let uploaded = workflow
        .upload("ana", "acta.txt", b"case document")
        .unwrap();
    assert_eq!(uploaded.version.get(), 1);
    assert_eq!(uploaded.name, "acta.txt");
    assert!(!uploaded.sealed);

    let sealed = workflow.seal("ana", uploaded.id).unwrap();
    assert!(sealed.sealed);

    let report = workflow.verify("ana", uploaded.id).unwrap();
    assert_eq!(report.verdict.to_string(), "valid");

    let package = workflow.export_evidence("ana", uploaded.id).unwrap();
    assert_eq!(package.archive, b"archive bytes");
    assert_eq!(package.file_name, "acta.txt-evidence.zip");

    assert_eq!(
        workflow.verify_audit().unwrap(),
        ChainVerification::Valid { entries: 4 }
    );
}

#[test]
fn an_unsealed_document_cannot_be_verified_or_exported() {
    let workflow = support::workflow();
    let uploaded = workflow
        .upload("ana", "acta.txt", b"case document")
        .unwrap();

    assert!(matches!(
        workflow.verify("ana", uploaded.id),
        Err(ApplicationError::DocumentNotSealed(_))
    ));
    assert!(matches!(
        workflow.export_evidence("ana", uploaded.id),
        Err(ApplicationError::DocumentNotSealed(_))
    ));
}

#[test]
fn a_missing_document_is_reported_consistently() {
    let workflow = support::workflow();

    assert!(matches!(
        workflow.seal("ana", DocumentId::new()),
        Err(ApplicationError::DocumentNotFound(_))
    ));
}

#[test]
fn a_document_cannot_be_sealed_twice() {
    let workflow = support::workflow();
    let uploaded = workflow
        .upload("ana", "acta.txt", b"case document")
        .unwrap();
    workflow.seal("ana", uploaded.id).unwrap();

    assert!(matches!(
        workflow.seal("ana", uploaded.id),
        Err(ApplicationError::DocumentAlreadySealed(_))
    ));
}
