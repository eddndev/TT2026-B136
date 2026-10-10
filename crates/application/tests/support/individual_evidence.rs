use std::{io::Read, sync::Mutex};

use application::documents::{DocumentRecord, IndividualSigningEvidence, SealedEvidence};
use domain::{
    cases::CaseId,
    crypto::{
        CertificateSummary, CredentialCertificate, CredentialCheck, CredentialFailure,
        CredentialTrustInspection, DocumentHasher, DocumentId, DocumentVersion, DocumentVersionRef,
        InternalDeclarationVerifier, Sha256Digest, Signature,
    },
    document_signing::{
        DocumentConsent, SignerAccount, SigningBindingMaterial, SigningBindingStatement,
    },
    identity::{Role, UserId},
    DomainError,
};
use uuid::Uuid;

pub const LEAF: &[u8] = b"canonical-synthetic-leaf";
pub const ROOT: &[u8] = b"canonical-synthetic-root";
pub const FIRST_CRL: &[u8] = b"canonical-synthetic-crl-one";
pub const SECOND_CRL: &[u8] = b"canonical-synthetic-crl-two";

pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        // Deterministic port double; these application tests exercise no RSA.
        let mut value = [0_u8; 32];
        for (index, byte) in bytes.iter().enumerate() {
            value[index % 32] = value[index % 32]
                .wrapping_add(*byte)
                .wrapping_add(index as u8);
        }
        Sha256Digest::from_array(value)
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("historical evidence validates bounded public bytes");
    }
}

pub fn signature(bytes: &[u8]) -> Vec<u8> {
    Hasher.hash_bytes(bytes).as_bytes().repeat(12)
}

#[derive(Default)]
pub struct Verifier {
    pub corruption: u8,
    pub calls: Mutex<Vec<i64>>,
}

impl InternalDeclarationVerifier for Verifier {
    fn inspect_certificate(
        &self,
        bytes: &[u8],
    ) -> Result<CredentialCertificate, CredentialFailure> {
        if bytes != LEAF {
            return Err(CredentialFailure::MalformedCertificate);
        }
        Ok(CredentialCertificate {
            der: LEAF.to_vec(),
            fingerprint: Hasher.hash_bytes(LEAF),
            summary: CertificateSummary {
                subject: "Synthetic holder".into(),
                issuer: "Synthetic root".into(),
                serial_hex: "1000".into(),
                not_before_unix: 0,
                not_after_unix: 1000,
            },
        })
    }

    fn inspect_trust(
        &self,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialTrustInspection, CredentialFailure> {
        if root != ROOT {
            return Err(CredentialFailure::UntrustedIssuer);
        }
        let (number, start, end) = if crl == FIRST_CRL {
            (1, 1, 200)
        } else if crl == SECOND_CRL {
            (2, 201, 400)
        } else {
            return Err(CredentialFailure::MalformedCrl);
        };
        if at < start || at > end {
            return Err(CredentialFailure::CrlExpired);
        }
        Ok(CredentialTrustInspection {
            root_der: ROOT.to_vec(),
            crl_der: crl.to_vec(),
            root_fingerprint: Hasher.hash_bytes(ROOT),
            crl_digest: Hasher.hash_bytes(crl),
            crl_number: number,
            crl_this_update: start,
            crl_next_update: end,
            valid_from: start,
            valid_until: end,
        })
    }

    fn verify(
        &self,
        digest: &Sha256Digest,
        certificate: &[u8],
        signature: &Signature,
        root: &[u8],
        crl: &[u8],
        at: i64,
    ) -> Result<CredentialCheck, CredentialFailure> {
        self.calls.lock().unwrap().push(at);
        if signature.as_bytes() != digest.as_bytes().repeat(12) {
            return Err(CredentialFailure::InvalidSignature);
        }
        let certificate = self.inspect_certificate(certificate)?;
        let trust = self.inspect_trust(root, crl, at)?;
        let mut check = CredentialCheck {
            certificate,
            valid_from: trust.valid_from,
            valid_until: trust.valid_until,
            trust,
            statement_digest: *digest,
            signature: signature.clone(),
            checked_at: at,
        };
        match self.corruption {
            1 => check.certificate.fingerprint = Sha256Digest::from_array([9; 32]),
            2 => check.trust.root_fingerprint = Sha256Digest::from_array([9; 32]),
            3 => check.trust.crl_digest = Sha256Digest::from_array([9; 32]),
            4 => check.checked_at += 1,
            5 => check.statement_digest = Sha256Digest::from_array([9; 32]),
            6 => check.signature = Signature::from_bytes(vec![9; 384]).unwrap(),
            7 => check.valid_until = at - 1,
            8 => check.certificate.der = b"different-der".to_vec(),
            9 => check.trust.crl_der = b"different-crl".to_vec(),
            _ => {}
        }
        Ok(check)
    }
}

pub fn fixture() -> (DocumentRecord, SealedEvidence, IndividualSigningEvidence) {
    let account = UserId::from_uuid(Uuid::from_u128(1));
    let binding_material = SigningBindingMaterial::new(
        Uuid::from_u128(2),
        Uuid::from_u128(3),
        Hasher.hash_bytes(ROOT),
        Hasher.hash_bytes(LEAF),
        1,
    )
    .unwrap();
    let consent_material = SigningBindingMaterial::new(
        Uuid::from_u128(2),
        Uuid::from_u128(3),
        Hasher.hash_bytes(ROOT),
        Hasher.hash_bytes(LEAF),
        2,
    )
    .unwrap();
    let registration = SigningBindingStatement::new(
        SignerAccount::new(account, Role::Litigator, true, 4, 2).unwrap(),
        account,
        binding_material,
    )
    .unwrap();
    let record = DocumentRecord::pending(
        DocumentId::from_uuid(Uuid::from_u128(4)),
        DocumentVersion::new(3).unwrap(),
        "contract.pdf".into(),
        Hasher.hash_bytes(b"PDF fixture"),
        vec![1],
    )
    .unwrap();
    let pdf_signature = vec![8_u8; 384];
    let consent = DocumentConsent::new(
        SignerAccount::new(account, Role::Litigator, true, 5, 3).unwrap(),
        consent_material,
        Uuid::from_u128(5),
        CaseId::from_uuid(Uuid::from_u128(6)),
        DocumentVersionRef {
            id: record.id,
            version: record.version,
        },
        record.digest,
        Hasher.hash_bytes(&pdf_signature),
    )
    .unwrap();
    let sealed = serde_json::from_value(serde_json::json!({
        "signature": pdf_signature,
        "timestamp_token": [1],
        "signer_certificate_pem": LEAF,
        "issuer_certificate_pem": ROOT,
        "crl_pem": SECOND_CRL,
        "tsa_chain_pem": null,
        "openssl_version": "synthetic",
    }))
    .unwrap();
    let binding_statement = registration.canonical_bytes().to_vec();
    let consent_statement = consent.canonical_bytes().to_vec();
    let evidence = IndividualSigningEvidence {
        format_version: 1,
        binding_signature: signature(&binding_statement),
        binding_statement,
        binding_root_der: ROOT.to_vec(),
        binding_crl_der: FIRST_CRL.to_vec(),
        binding_registered_at: 100,
        consent_signature: signature(&consent_statement),
        consent_statement,
        accepted_at: 300,
    };
    (record, sealed, evidence)
}

pub fn resign(evidence: &mut IndividualSigningEvidence) {
    evidence.binding_signature = signature(&evidence.binding_statement);
    evidence.consent_signature = signature(&evidence.consent_statement);
}
