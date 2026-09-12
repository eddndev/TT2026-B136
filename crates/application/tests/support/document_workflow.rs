use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

use application::documents::{
    DocumentProcessor, DocumentProcessorPorts, DocumentRecord, DocumentRepository,
    DocumentWorkflowPorts, EvidenceMaterial, LocalDocumentWorkflow,
};
use application::ApplicationError;
use domain::audit::{chain_digest, AuditEvent, AuditLog, ChainedEvent, GENESIS_PREVIOUS};
use domain::clock::{Clock, OffsetDateTime};
use domain::crypto::{
    ArchiveEntry, ArchiveWriter, AuthenticatedCipher, CertificateSummary, CertificateValidation,
    CertificateValidator, DocumentHasher, DocumentId, DocumentSigner, KeyManager, SealedPayload,
    Sha256Digest, Signature, SignatureVerification, SignatureVerifier, TimestampService,
    TimestampVerification, TimestampVerifier, WrappedDek,
};
use domain::DomainError;
use zeroize::Zeroizing;

#[derive(Default)]
pub struct MemoryRepository {
    records: Mutex<HashMap<DocumentId, DocumentRecord>>,
}

impl DocumentRepository for MemoryRepository {
    fn insert(&self, record: DocumentRecord) -> Result<(), ApplicationError> {
        let mut records = self.records.lock().unwrap();
        if records.contains_key(&record.id) {
            return Err(ApplicationError::DocumentAlreadyExists(
                record.id.to_string(),
            ));
        }
        records.insert(record.id, record);
        Ok(())
    }

    fn replace(&self, record: DocumentRecord) -> Result<(), ApplicationError> {
        let mut records = self.records.lock().unwrap();
        if !records.contains_key(&record.id) {
            return Err(ApplicationError::DocumentNotFound(record.id.to_string()));
        }
        records.insert(record.id, record);
        Ok(())
    }

    fn find(&self, id: DocumentId) -> Result<Option<DocumentRecord>, ApplicationError> {
        Ok(self.records.lock().unwrap().get(&id).cloned())
    }
}

#[derive(Clone, Copy)]
struct TestHasher;

impl DocumentHasher for TestHasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        let mut bytes = [0u8; 32];
        for (index, byte) in data.iter().enumerate() {
            let slot = index % bytes.len();
            bytes[slot] = bytes[slot].wrapping_add(*byte).wrapping_add(index as u8);
        }
        Sha256Digest::from_array(bytes)
    }

    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|error| DomainError::StreamRead {
                message: error.to_string(),
            })?;
        Ok(self.hash_bytes(&bytes))
    }
}

struct TestCipher;

impl AuthenticatedCipher for TestCipher {
    fn seal(
        &self,
        _key: &[u8],
        _aad: &[u8],
        plaintext: &[u8],
    ) -> Result<SealedPayload, DomainError> {
        let mut bytes = vec![0u8; 12];
        bytes.extend_from_slice(plaintext);
        bytes.extend_from_slice(&[0u8; 16]);
        SealedPayload::from_bytes(bytes)
    }

    fn open(
        &self,
        _key: &[u8],
        _aad: &[u8],
        payload: &SealedPayload,
    ) -> Result<Vec<u8>, DomainError> {
        let bytes = payload.as_bytes();
        Ok(bytes[12..bytes.len() - 16].to_vec())
    }
}

struct TestKeys;

impl KeyManager for TestKeys {
    fn generate_dek(&self) -> Result<Vec<u8>, DomainError> {
        Ok(vec![0x11; 32])
    }

    fn wrap_dek(&self, _kek: &[u8], _dek: &[u8]) -> Result<WrappedDek, DomainError> {
        WrappedDek::from_bytes(vec![0x22; 60])
    }

    fn unwrap_dek(&self, _kek: &[u8], _wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError> {
        Ok(vec![0x11; 32])
    }

    fn rewrap_dek(
        &self,
        _old_kek: &[u8],
        _new_kek: &[u8],
        wrapped: &WrappedDek,
    ) -> Result<WrappedDek, DomainError> {
        WrappedDek::from_bytes(wrapped.as_bytes().to_vec())
    }
}

struct TestSigner;

impl DocumentSigner for TestSigner {
    fn sign(&self, digest: &Sha256Digest) -> Result<Signature, DomainError> {
        Signature::from_bytes(digest.as_bytes().to_vec())
    }
}

struct TestSignatureVerifier;

impl SignatureVerifier for TestSignatureVerifier {
    fn verify(
        &self,
        digest: &Sha256Digest,
        signature: &Signature,
        _key_material: &[u8],
    ) -> Result<SignatureVerification, DomainError> {
        if signature.as_bytes() == digest.as_bytes() {
            Ok(SignatureVerification::Valid)
        } else {
            Ok(SignatureVerification::Invalid(
                domain::crypto::SignatureRejection::MismatchedDocumentOrKey,
            ))
        }
    }
}

struct TestTimestamp;

impl TimestampService for TestTimestamp {
    fn request(&self, digest: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        Ok(digest.as_bytes().to_vec())
    }
}

impl TimestampVerifier for TestTimestamp {
    fn verify(
        &self,
        token: &[u8],
        expected: &Sha256Digest,
        _trust_anchor_pem: &[u8],
    ) -> Result<TimestampVerification, DomainError> {
        if token == expected.as_bytes() {
            Ok(TimestampVerification::Valid {
                generated_at: "2026-08-17T12:00:00Z".to_string(),
            })
        } else {
            Ok(TimestampVerification::ImprintMismatch)
        }
    }
}

struct TestCertificateValidator;

impl CertificateValidator for TestCertificateValidator {
    fn validate(
        &self,
        _certificate: &[u8],
        _issuer: &[u8],
        _crl: Option<&[u8]>,
        _unix_seconds: i64,
    ) -> Result<CertificateValidation, DomainError> {
        Ok(CertificateValidation::Valid)
    }

    fn inspect(&self, _certificate: &[u8]) -> Result<CertificateSummary, DomainError> {
        Ok(CertificateSummary {
            subject: "subject".to_string(),
            issuer: "issuer".to_string(),
            serial_hex: "01".to_string(),
            not_before_unix: 0,
            not_after_unix: i64::MAX,
        })
    }
}

struct TestArchiver;

impl ArchiveWriter for TestArchiver {
    fn write_archive(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError> {
        assert!(!entries.is_empty());
        Ok(b"archive bytes".to_vec())
    }
}

struct TestAuditLog {
    entries: Vec<ChainedEvent>,
}

impl AuditLog for TestAuditLog {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        let sequence = self.entries.len() as u64;
        let previous = self
            .entries
            .last()
            .map(|entry| entry.chain)
            .unwrap_or(GENESIS_PREVIOUS);
        let event = AuditEvent::new(sequence, timestamp, actor, action, resource);
        let chain = chain_digest(&TestHasher, &previous, &event)?;
        let entry = ChainedEvent { event, chain };
        self.entries.push(entry.clone());
        Ok(entry)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        Ok(self.entries.clone())
    }
}

pub struct TestClock;

impl Clock for TestClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_787_000_000).unwrap()
    }
}

pub fn processor_ports() -> DocumentProcessorPorts {
    DocumentProcessorPorts {
        hasher: Box::new(TestHasher),
        cipher: Box::new(TestCipher),
        keys: Box::new(TestKeys),
        signer: Box::new(TestSigner),
        timestamp_service: Box::new(TestTimestamp),
        signature_verifier: Box::new(TestSignatureVerifier),
        certificate_validator: Box::new(TestCertificateValidator),
        timestamp_verifier: Box::new(TestTimestamp),
        archiver: Box::new(TestArchiver),
    }
}

pub fn evidence_material() -> EvidenceMaterial {
    EvidenceMaterial {
        signer_certificate_pem: b"signer pem".to_vec(),
        issuer_certificate_pem: b"issuer pem".to_vec(),
        crl_pem: b"crl pem".to_vec(),
        tsa_chain_pem: Some(b"tsa chain".to_vec()),
        openssl_version: "OpenSSL test".to_string(),
    }
}

pub fn processor() -> DocumentProcessor {
    DocumentProcessor::new(
        processor_ports(),
        evidence_material(),
        Zeroizing::new(vec![0x44; 32]),
    )
    .unwrap()
}

pub fn workflow() -> LocalDocumentWorkflow {
    let crypto = processor_ports();
    let ports = DocumentWorkflowPorts {
        repository: Arc::new(MemoryRepository::default()),
        audit_log: Box::new(TestAuditLog {
            entries: Vec::new(),
        }),
        clock: Box::new(TestClock),
        hasher: crypto.hasher,
        cipher: crypto.cipher,
        keys: crypto.keys,
        signer: crypto.signer,
        timestamp_service: crypto.timestamp_service,
        signature_verifier: crypto.signature_verifier,
        certificate_validator: crypto.certificate_validator,
        timestamp_verifier: crypto.timestamp_verifier,
        archiver: crypto.archiver,
    };
    LocalDocumentWorkflow::new(ports, evidence_material(), Zeroizing::new(vec![0x44; 32])).unwrap()
}
