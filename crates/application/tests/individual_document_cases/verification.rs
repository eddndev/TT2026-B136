use crate::{document_signing_support as bindings, individual_document_support::Harness, proofs};
use application::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    document_signing::{CurrentSigningStatus, SigningVerificationContext},
    documents::{DocumentProcessor, DocumentRecord},
    verification::Verdict,
    ApplicationError,
};
use domain::{
    crypto::{
        CredentialCertificate, CredentialCheck, CredentialFailure, CredentialTrustInspection,
        DocumentVersionRef, InternalDeclarationVerifier, Sha256Digest, Signature,
        SignatureVerification, SignatureVerifier,
    },
    document_signing::DocumentConsent,
    identity::Role,
    DomainError,
};
use std::sync::Arc;
use zeroize::Zeroizing;

struct Content;
impl SignatureVerifier for Content {
    fn verify(
        &self,
        _: &Sha256Digest,
        _: &Signature,
        _: &[u8],
    ) -> Result<SignatureVerification, DomainError> {
        Ok(SignatureVerification::Valid)
    }
}
struct CurrentVerifier(bool);
impl InternalDeclarationVerifier for CurrentVerifier {
    fn inspect_certificate(
        &self,
        bytes: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        proofs::Verifier::default().inspect_certificate(bytes)
    }
    fn inspect_trust(
        &self,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialTrustInspection, CredentialFailure> {
        proofs::Verifier::default().inspect_trust(root, crl, at)
    }
    fn verify(
        &self,
        digest: &Sha256Digest,
        cert: &[u8],
        signature: &Signature,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialCheck, CredentialFailure> {
        if self.0 && at == 350 {
            return Err(CredentialFailure::Revoked);
        }
        proofs::Verifier::default().verify(digest, cert, signature, root, crl, at)
    }
}
fn input() -> (DocumentRecord, DocumentConsent) {
    let (plain, mut evidence, proof) = proofs::fixture();
    let consent = DocumentConsent::from_canonical_bytes(&proof.consent_statement).unwrap();
    let mut record = crate::crypto::processor()
        .prepare_version(plain.id, plain.version, &plain.name, b"PDF fixture")
        .unwrap();
    evidence.timestamp_token = record.digest.as_bytes().to_vec();
    evidence.individual = Some(proof);
    record.seal(evidence).unwrap();
    (record, consent)
}
fn trust() -> CredentialTrustSnapshot {
    CredentialTrustSnapshot {
        deployment_id: uuid::Uuid::from_u128(2),
        revision: CredentialTrustRevision::new(3).unwrap(),
        inspection: proofs::Verifier::default()
            .inspect_trust(proofs::ROOT, proofs::SECOND_CRL, 350)
            .unwrap(),
        published_at: bindings::at(301),
        published_by: "local authority".into(),
    }
}
fn fixture(
    role: Role,
    revoked: bool,
    current: Option<CredentialTrustSnapshot>,
    legacy: bool,
    change: bool,
) -> (Harness, DocumentConsent) {
    let mut harness = Harness::new();
    {
        let mut state = harness.binding.state.lock().unwrap();
        state.now = 350;
        state.principal.as_mut().unwrap().role = role;
    }
    let (mut record, consent) = input();
    if legacy {
        record.evidence.as_mut().unwrap().individual = None;
    }
    let mut ports = crate::crypto::processor_ports();
    ports.signature_verifier = Box::new(Content);
    harness.processor = Arc::new(
        DocumentProcessor::new(
            ports,
            crate::crypto::evidence_material(),
            Zeroizing::new(vec![0x44; 32]),
        )
        .unwrap()
        .with_individual_verifier(Box::new(CurrentVerifier(revoked))),
    );
    let observed = harness.binding.state.clone();
    harness
        .store
        .expect_verification_context()
        .returning(move |principal, case, reference| {
            assert_eq!(principal.role, role);
            assert_eq!(case, consent.case());
            assert_eq!(reference, consent.document());
            if change {
                observed.lock().unwrap().principal.as_mut().unwrap().email =
                    "changed@example.test".into();
            }
            Ok(SigningVerificationContext {
                signing: if legacy {
                    None
                } else {
                    Some(application::document_signing::SigningReceipt {
                        consent,
                        evidence: record.evidence.clone().unwrap(),
                        accepted_at: bindings::at(300).replace_nanosecond(123456789).unwrap(),
                        manifest: None,
                    })
                },
                record: record.clone(),
                trust: current.clone(),
                binding_withdrawn_at: Some(bindings::at(320)),
            })
        });
    (harness, consent)
}
#[test]
fn historical_validity_survives_current_revocation_and_binding_withdrawal_for_read_roles() {
    for role in [Role::Owner, Role::Litigator, Role::Paralegal] {
        let (harness, consent) = fixture(role, true, Some(trust()), false, false);
        let result = harness
            .service()
            .verify("session", consent.case(), consent.document())
            .unwrap();
        assert_eq!(result.historical.verdict, Verdict::Valid);
        assert_eq!(
            result.certificate_current,
            CurrentSigningStatus::Rejected(CredentialFailure::Revoked)
        );
        assert_eq!(result.binding_withdrawn_at, Some(bindings::at(320)));
        assert_eq!(result.signing.unwrap().consent, consent);
    }
}
#[test]
fn current_successor_trust_missing_trust_and_legacy_are_distinct() {
    for (trust, legacy, status) in [
        (Some(trust()), false, CurrentSigningStatus::Valid),
        (None, false, CurrentSigningStatus::TrustUnavailable),
        (None, true, CurrentSigningStatus::NotIndividual),
    ] {
        let (harness, consent) = fixture(Role::Owner, false, trust, legacy, false);
        let result = harness
            .service()
            .verify("session", consent.case(), consent.document())
            .unwrap();
        assert_eq!(result.certificate_current, status);
        assert_eq!(result.signing.is_none(), legacy);
        if legacy {
            assert!(result.binding_withdrawn_at.is_none());
        }
    }
}
#[test]
fn verification_reauthenticates_complete_principal_and_denies_client() {
    let (harness, consent) = fixture(Role::Owner, false, None, false, true);
    assert!(matches!(
        harness
            .service()
            .verify("session", consent.case(), consent.document()),
        Err(ApplicationError::InvalidSession)
    ));
    let harness = Harness::new();
    harness
        .binding
        .state
        .lock()
        .unwrap()
        .principal
        .as_mut()
        .unwrap()
        .role = Role::Client;
    assert!(matches!(
        harness.service().verify(
            "session",
            consent.case(),
            DocumentVersionRef {
                id: consent.document().id,
                version: consent.document().version
            }
        ),
        Err(ApplicationError::PermissionDenied)
    ));
}

#[test]
fn historical_status_preserves_the_original_receipt_precision() {
    let (harness, consent) = fixture(Role::Owner, false, Some(trust()), false, false);
    let result = harness
        .service()
        .verify("session", consent.case(), consent.document())
        .unwrap();
    assert_eq!(
        result.signing.unwrap().accepted_at,
        bindings::at(300).replace_nanosecond(123456789).unwrap()
    );
}
