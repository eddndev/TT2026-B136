#[allow(dead_code)]
mod case_document_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
#[allow(dead_code)]
mod document_content_support;

use application::{
    document_content::DocumentContentWorkflow, document_integrity::DocumentIntegrityFailure,
    documents::DocumentProcessor, ApplicationError,
};
use case_document_support::MockStore;
use document_content_support::{identity, principal, receipt, reference, service, MockIncidents};
use domain::{
    cases::CaseId,
    crypto::{document_aad, AuthenticatedCipher, SealedPayload},
    DomainError,
};

struct RejectedCipher(Vec<u8>);
impl AuthenticatedCipher for RejectedCipher {
    fn seal(&self, _: &[u8], _: &[u8], _: &[u8]) -> Result<SealedPayload, DomainError> {
        panic!("content reads must not encrypt")
    }
    fn open(&self, _: &[u8], aad: &[u8], _: &SealedPayload) -> Result<Vec<u8>, DomainError> {
        assert_eq!(
            aad, self.0,
            "decryption must use the exact requested identity and version"
        );
        Err(DomainError::AuthenticationFailed)
    }
}

#[test]
fn malformed_vault_digest_and_aes_rejections_record_failure_without_success_or_bytes() {
    for failure in [
        DocumentIntegrityFailure::MalformedVault,
        DocumentIntegrityFailure::DigestMismatch,
        DocumentIntegrityFailure::AuthenticationFailed,
    ] {
        let mut record = crypto::processor()
            .prepare("private.bin", b"protected bytes")
            .unwrap();
        let requested = reference(&record);
        let mut ports = crypto::processor_ports();
        match failure {
            DocumentIntegrityFailure::MalformedVault => record.vault[0] ^= 1,
            DocumentIntegrityFailure::DigestMismatch => {
                record.digest = domain::crypto::Sha256Digest::from_array([9; 32])
            }
            DocumentIntegrityFailure::AuthenticationFailed => {
                ports.cipher = Box::new(RejectedCipher(
                    document_aad(record.id, record.version).to_vec(),
                ));
            }
            _ => unreachable!(),
        }
        let reader = DocumentProcessor::new(
            ports,
            crypto::evidence_material(),
            zeroize::Zeroizing::new(vec![0x44; 32]),
        )
        .unwrap();
        let case = CaseId::new();
        let principal = principal();
        let actor = principal.id;
        let captured = record.clone();
        let mut incidents = MockIncidents::new();
        incidents
            .expect_record_rejection()
            .times(1)
            .withf(move |observation| {
                observation.case_id == case
                    && observation.requester == actor
                    && observation.record == captured
                    && observation.failure == failure
            })
            .returning(|observation| Ok(receipt(observation)));
        let mut store = MockStore::new();
        store
            .expect_load()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(record.clone()));
        let result = service(store, identity(principal, 2), reader, incidents)
            .content_version("session", case, requested);
        assert!(
            matches!(result, Err(ApplicationError::DocumentContentValidationFailed(found)) if found == failure)
        );
    }
}

#[test]
fn revocation_snapshot_change_and_audit_failure_prevent_release_after_decryption() {
    for mode in 0..3 {
        let record = crypto::processor()
            .prepare("private.bin", b"protected bytes")
            .unwrap();
        let requested = reference(&record);
        let mut incidents = MockIncidents::new();
        if mode == 1 {
            incidents
                .expect_record_rejection()
                .times(1)
                .withf(|observation| {
                    observation.failure == DocumentIntegrityFailure::SnapshotChanged
                })
                .returning(|observation| Ok(receipt(observation)));
        }
        let mut store = MockStore::new();
        store
            .expect_load()
            .times(1)
            .returning(move |_, _, _, _, _| Ok(record.clone()));
        store
            .expect_record_access()
            .times(1)
            .returning(move |_, _, _, _, _| {
                Err(match mode {
                    0 => ApplicationError::PermissionDenied,
                    1 => ApplicationError::DocumentContentValidationFailed(
                        DocumentIntegrityFailure::SnapshotChanged,
                    ),
                    _ => ApplicationError::Port("audit commit failed".into()),
                })
            });
        let result = service(
            store,
            identity(principal(), 2),
            crypto::processor(),
            incidents,
        )
        .content_version("session", CaseId::new(), requested);
        match mode {
            0 => assert!(matches!(result, Err(ApplicationError::PermissionDenied))),
            1 => assert!(matches!(
                result,
                Err(ApplicationError::DocumentContentValidationFailed(
                    DocumentIntegrityFailure::SnapshotChanged
                ))
            )),
            _ => assert!(matches!(result, Err(ApplicationError::Port(_)))),
        }
    }
}

#[test]
fn incident_commit_failure_is_technical_and_never_claims_durable_notification() {
    let mut record = crypto::processor()
        .prepare("private.bin", b"protected bytes")
        .unwrap();
    record.vault[0] ^= 1;
    let requested = reference(&record);
    let mut store = MockStore::new();
    store
        .expect_load()
        .times(1)
        .returning(move |_, _, _, _, _| Ok(record.clone()));
    let mut incidents = MockIncidents::new();
    incidents
        .expect_record_rejection()
        .times(1)
        .returning(|_| Err(ApplicationError::Port("incident transaction failed".into())));
    let result = service(
        store,
        identity(principal(), 2),
        crypto::processor(),
        incidents,
    )
    .content_version("session", CaseId::new(), requested);
    assert!(matches!(result, Err(ApplicationError::Port(_))));
}

#[test]
fn a_read_size_limit_is_not_an_integrity_incident() {
    let mut record = crypto::processor()
        .prepare("large.bin", b"initial")
        .unwrap();
    record.vault.resize(16 * 1024 * 1024 + 98, 0);
    let requested = reference(&record);
    let mut store = MockStore::new();
    store
        .expect_load()
        .times(1)
        .returning(move |_, _, _, _, _| Ok(record.clone()));
    let result = service(
        store,
        identity(principal(), 1),
        crypto::processor(),
        MockIncidents::new(),
    )
    .content_version("session", CaseId::new(), requested);
    assert!(matches!(
        result,
        Err(ApplicationError::DocumentContentTooLarge)
    ));
}
