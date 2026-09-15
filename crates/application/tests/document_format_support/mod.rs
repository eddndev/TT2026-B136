use application::documents::{DocumentProcessor, DocumentProcessorPorts};
use domain::{crypto::*, DomainError};
use std::{
    io::Read,
    sync::{Arc, Mutex},
};
use zeroize::Zeroizing;

#[derive(Default)]
pub struct Observations {
    pub events: Vec<&'static str>,
    pub pointers: Vec<usize>,
    pub aad: Vec<Vec<u8>>,
}

#[derive(Clone)]
struct Probe {
    ports: Arc<DocumentProcessorPorts>,
    observations: Arc<Mutex<Observations>>,
}

impl Probe {
    fn event(&self, value: &'static str) {
        self.observations.lock().unwrap().events.push(value);
    }
}

pub fn processor() -> (DocumentProcessor, Arc<Mutex<Observations>>) {
    let observations = Arc::new(Mutex::new(Observations::default()));
    let probe = Probe {
        ports: Arc::new(super::crypto::processor_ports()),
        observations: observations.clone(),
    };
    let mut ports = super::crypto::processor_ports();
    ports.cipher = Box::new(probe.clone());
    ports.keys = Box::new(probe.clone());
    ports.hasher = Box::new(probe.clone());
    ports.signature_verifier = Box::new(probe.clone());
    ports.timestamp_verifier = Box::new(probe.clone());
    ports.certificate_validator = Box::new(probe.clone());
    ports.signer = Box::new(probe.clone());
    ports.timestamp_service = Box::new(probe);
    (
        DocumentProcessor::new(
            ports,
            super::crypto::evidence_material(),
            Zeroizing::new(vec![1; 32]),
        )
        .unwrap(),
        observations,
    )
}

impl AuthenticatedCipher for Probe {
    fn seal(&self, _: &[u8], _: &[u8], _: &[u8]) -> Result<SealedPayload, DomainError> {
        panic!("validation must not encrypt")
    }
    fn open(
        &self,
        key: &[u8],
        aad: &[u8],
        payload: &SealedPayload,
    ) -> Result<Vec<u8>, DomainError> {
        self.event("open");
        let bytes = self.ports.cipher.open(key, aad, payload)?;
        let mut observations = self.observations.lock().unwrap();
        observations.pointers.push(bytes.as_ptr() as usize);
        observations.aad.push(aad.to_vec());
        Ok(bytes)
    }
}

impl KeyManager for Probe {
    fn generate_dek(&self) -> Result<Vec<u8>, DomainError> {
        panic!("validation must not generate a key")
    }
    fn wrap_dek(&self, _: &[u8], _: &[u8]) -> Result<WrappedDek, DomainError> {
        panic!("validation must not wrap a key")
    }
    fn rewrap_dek(&self, _: &[u8], _: &[u8], _: &WrappedDek) -> Result<WrappedDek, DomainError> {
        panic!("validation must not rotate a key")
    }
    fn unwrap_dek(&self, kek: &[u8], wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError> {
        self.event("unwrap");
        self.ports.keys.unwrap_dek(kek, wrapped)
    }
}

impl DocumentHasher for Probe {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        self.event("hash");
        self.ports.hasher.hash_bytes(bytes)
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("validation must hash the already decrypted bytes")
    }
}

impl SignatureVerifier for Probe {
    fn verify(
        &self,
        digest: &Sha256Digest,
        signature: &Signature,
        material: &[u8],
    ) -> Result<SignatureVerification, DomainError> {
        self.event("signature");
        self.ports
            .signature_verifier
            .verify(digest, signature, material)
    }
}

impl TimestampVerifier for Probe {
    fn verify(
        &self,
        token: &[u8],
        digest: &Sha256Digest,
        anchor: &[u8],
    ) -> Result<TimestampVerification, DomainError> {
        self.event("timestamp");
        self.ports.timestamp_verifier.verify(token, digest, anchor)
    }
}

impl CertificateValidator for Probe {
    fn validate(
        &self,
        certificate: &[u8],
        issuer: &[u8],
        crl: Option<&[u8]>,
        at: i64,
    ) -> Result<CertificateValidation, DomainError> {
        self.event("certificate");
        self.ports
            .certificate_validator
            .validate(certificate, issuer, crl, at)
    }
    fn inspect(&self, _: &[u8]) -> Result<CertificateSummary, DomainError> {
        panic!("validation must not inspect a display summary")
    }
}

impl DocumentSigner for Probe {
    fn sign(&self, _: &Sha256Digest) -> Result<Signature, DomainError> {
        panic!("validation must not sign")
    }
}

impl TimestampService for Probe {
    fn request(&self, _: &Sha256Digest) -> Result<Vec<u8>, DomainError> {
        panic!("validation must not request a timestamp")
    }
}
