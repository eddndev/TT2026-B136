//! Cryptographic preparation shared by online and offline document workflows.

use domain::audit::{verify_chain, ChainVerification, ChainedEvent};
use domain::crypto::keys::KEY_ENCRYPTION_KEY_LEN;
use domain::crypto::{
    ArchiveEntry, ArchiveWriter, AuthenticatedCipher, CertificateValidator, DocumentHasher,
    DocumentId, DocumentSigner, DocumentVersion, KeyManager, Signature, SignatureVerification,
    SignatureVerifier, TimestampService, TimestampVerification, TimestampVerifier,
};
use zeroize::Zeroizing;

use super::validation::{
    plaintext_with_ports, validate_record_with_ports, DocumentValidationPorts,
};
use super::{DocumentRecord, EvidenceExport, SealedEvidence};
use crate::evidence::{export_with_ports, EvidenceRequest};
use crate::vault::encrypt_with_ports;
use crate::verification::{
    verify_with_ports, TimestampEvidence, VerificationReport, VerifyDocumentRequest,
};
use crate::ApplicationError;

/// Cryptographic adapters only; no persistence or transaction is held here.
pub struct DocumentProcessorPorts {
    pub hasher: Box<dyn DocumentHasher + Send + Sync>,
    pub cipher: Box<dyn AuthenticatedCipher + Send + Sync>,
    pub keys: Box<dyn KeyManager + Send + Sync>,
    pub signer: Box<dyn DocumentSigner + Send + Sync>,
    pub timestamp_service: Box<dyn TimestampService + Send + Sync>,
    pub signature_verifier: Box<dyn SignatureVerifier + Send + Sync>,
    pub certificate_validator: Box<dyn CertificateValidator + Send + Sync>,
    pub timestamp_verifier: Box<dyn TimestampVerifier + Send + Sync>,
    pub archiver: Box<dyn ArchiveWriter + Send + Sync>,
}

/// Public evidence captured at sealing time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceMaterial {
    pub signer_certificate_pem: Vec<u8>,
    pub issuer_certificate_pem: Vec<u8>,
    pub crl_pem: Vec<u8>,
    pub tsa_chain_pem: Option<Vec<u8>>,
    pub openssl_version: String,
}

pub struct DocumentProcessor {
    ports: DocumentProcessorPorts,
    material: EvidenceMaterial,
    kek: Zeroizing<Vec<u8>>,
}

impl DocumentProcessor {
    pub fn new(
        ports: DocumentProcessorPorts,
        material: EvidenceMaterial,
        kek: Zeroizing<Vec<u8>>,
    ) -> Result<Self, ApplicationError> {
        if kek.len() != KEY_ENCRYPTION_KEY_LEN {
            return Err(ApplicationError::InvalidConfiguration(format!(
                "document kek must be {KEY_ENCRYPTION_KEY_LEN} bytes, got {}",
                kek.len()
            )));
        }
        validate_material(&material)?;
        Ok(Self {
            ports,
            material,
            kek,
        })
    }

    pub fn prepare(&self, name: &str, document: &[u8]) -> Result<DocumentRecord, ApplicationError> {
        ArchiveEntry::new(name.to_owned(), Vec::new())?;
        let id = DocumentId::new();
        let version = DocumentVersion::initial();
        let digest = self.ports.hasher.hash_bytes(document);
        let vault = encrypt_with_ports(
            self.ports.cipher.as_ref(),
            self.ports.keys.as_ref(),
            &self.kek,
            document,
            id,
            version,
        )?;
        DocumentRecord::pending(id, version, name.to_string(), digest, vault)
    }

    pub fn seal(&self, input: &DocumentRecord) -> Result<DocumentRecord, ApplicationError> {
        let mut record = input.clone();
        let id = record.id;
        if record.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(id.to_string()));
        }
        let _plaintext = self.plaintext(&record)?;
        let signature = self.ports.signer.sign(&record.digest)?;
        ensure_signature_matches(
            self.ports.signature_verifier.as_ref(),
            &record.digest,
            &signature,
            &self.material.signer_certificate_pem,
        )?;
        let token = self.ports.timestamp_service.request(&record.digest)?;
        let anchor = self
            .material
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&self.material.issuer_certificate_pem);
        ensure_timestamp_matches(
            self.ports.timestamp_verifier.as_ref(),
            &token,
            &record.digest,
            anchor,
        )?;
        let evidence = SealedEvidence {
            signature: signature.into_bytes(),
            timestamp_token: token,
            signer_certificate_pem: self.material.signer_certificate_pem.clone(),
            issuer_certificate_pem: self.material.issuer_certificate_pem.clone(),
            crl_pem: self.material.crl_pem.clone(),
            tsa_chain_pem: self.material.tsa_chain_pem.clone(),
            openssl_version: self.material.openssl_version.clone(),
        };
        record.seal(evidence)?;

        Ok(record)
    }

    pub fn verify(
        &self,
        record: &DocumentRecord,
        evaluation_unix: i64,
    ) -> Result<VerificationReport, ApplicationError> {
        let id = record.id;
        let plaintext = self.plaintext(record)?;
        let evidence = record
            .evidence
            .as_ref()
            .ok_or_else(|| ApplicationError::DocumentNotSealed(id.to_string()))?;
        let signature = Signature::from_bytes(evidence.signature.clone())?;
        let anchor = evidence
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&evidence.issuer_certificate_pem);
        let request = VerifyDocumentRequest {
            signature: &signature,
            signer_certificate_pem: &evidence.signer_certificate_pem,
            issuer_certificate_pem: &evidence.issuer_certificate_pem,
            crl_pem: Some(&evidence.crl_pem),
            timestamp: Some(TimestampEvidence {
                token: &evidence.timestamp_token,
                trust_anchor_pem: anchor,
            }),
            evaluation_unix,
        };
        let report = verify_with_ports(
            self.ports.hasher.as_ref(),
            self.ports.signature_verifier.as_ref(),
            self.ports.certificate_validator.as_ref(),
            self.ports.timestamp_verifier.as_ref(),
            &mut plaintext.as_slice(),
            &request,
        )?;

        Ok(report)
    }

    pub fn export_evidence(
        &self,
        record: &DocumentRecord,
    ) -> Result<EvidenceExport, ApplicationError> {
        let id = record.id;
        let plaintext = self.plaintext(record)?;
        let evidence = record
            .evidence
            .as_ref()
            .ok_or_else(|| ApplicationError::DocumentNotSealed(id.to_string()))?;
        let request = EvidenceRequest {
            document_name: &record.name,
            document: &plaintext,
            signature: &evidence.signature,
            timestamp_token: &evidence.timestamp_token,
            signer_certificate_pem: &evidence.signer_certificate_pem,
            issuer_certificate_pem: &evidence.issuer_certificate_pem,
            crl_pem: &evidence.crl_pem,
            tsa_chain_pem: evidence.tsa_chain_pem.as_deref(),
            openssl_version: &evidence.openssl_version,
        };
        let package = export_with_ports(
            self.ports.hasher.as_ref(),
            self.ports.archiver.as_ref(),
            &request,
        )?;

        Ok(EvidenceExport {
            archive: package.archive,
            file_name: format!("{}-evidence.zip", record.name),
            document_digest_hex: package.document_digest_hex,
        })
    }

    /// Validates historical bytes without signing, requesting a timestamp, or writing.
    pub fn validate_record(&self, record: &DocumentRecord) -> Result<(), ApplicationError> {
        validate_record_with_ports(record, &self.kek, &self.validation_ports())
    }

    pub fn verify_audit(
        &self,
        entries: &[ChainedEvent],
    ) -> Result<ChainVerification, ApplicationError> {
        Ok(verify_chain(self.ports.hasher.as_ref(), entries)?)
    }

    fn plaintext(&self, record: &DocumentRecord) -> Result<Zeroizing<Vec<u8>>, ApplicationError> {
        plaintext_with_ports(record, &self.kek, &self.validation_ports())
    }

    fn validation_ports(&self) -> DocumentValidationPorts<'_> {
        DocumentValidationPorts {
            cipher: self.ports.cipher.as_ref(),
            keys: self.ports.keys.as_ref(),
            hasher: self.ports.hasher.as_ref(),
            signature_verifier: self.ports.signature_verifier.as_ref(),
            certificate_validator: self.ports.certificate_validator.as_ref(),
            timestamp_verifier: self.ports.timestamp_verifier.as_ref(),
        }
    }
}

fn validate_material(material: &EvidenceMaterial) -> Result<(), ApplicationError> {
    if material.signer_certificate_pem.is_empty()
        || material.issuer_certificate_pem.is_empty()
        || material.crl_pem.is_empty()
        || material.openssl_version.trim().is_empty()
    {
        return Err(ApplicationError::InvalidConfiguration(
            "certificate, crl, and openssl evidence material must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn ensure_signature_matches(
    verifier: &(dyn SignatureVerifier + Send + Sync),
    digest: &domain::crypto::Sha256Digest,
    signature: &Signature,
    certificate: &[u8],
) -> Result<(), ApplicationError> {
    match verifier.verify(digest, signature, certificate)? {
        SignatureVerification::Valid => Ok(()),
        SignatureVerification::Invalid(cause) => Err(ApplicationError::InvalidConfiguration(
            format!("signing key does not match signer certificate: {cause}"),
        )),
    }
}

fn ensure_timestamp_matches(
    verifier: &(dyn TimestampVerifier + Send + Sync),
    token: &[u8],
    digest: &domain::crypto::Sha256Digest,
    anchor: &[u8],
) -> Result<(), ApplicationError> {
    match verifier.verify(token, digest, anchor)? {
        TimestampVerification::Valid { .. } => Ok(()),
        outcome => Err(ApplicationError::TimestampEvidenceRejected(format!(
            "{outcome:?}"
        ))),
    }
}
