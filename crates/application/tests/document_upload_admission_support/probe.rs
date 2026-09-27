use std::{io::Read, sync::Arc};

use application::documents::DocumentProcessor;
use domain::{crypto::*, DomainError};
use zeroize::Zeroizing;

use super::Trace;

struct Cipher {
    delegate: Box<dyn AuthenticatedCipher + Send + Sync>,
    trace: Trace,
}
impl AuthenticatedCipher for Cipher {
    fn seal(&self, key: &[u8], aad: &[u8], bytes: &[u8]) -> Result<SealedPayload, DomainError> {
        self.trace.lock().unwrap().push("encrypt");
        self.delegate.seal(key, aad, bytes)
    }
    fn open(&self, key: &[u8], aad: &[u8], value: &SealedPayload) -> Result<Vec<u8>, DomainError> {
        self.delegate.open(key, aad, value)
    }
}
struct Hasher {
    delegate: Box<dyn DocumentHasher + Send + Sync>,
    trace: Trace,
}
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        self.trace.lock().unwrap().push("hash");
        self.delegate.hash_bytes(bytes)
    }
    fn hash_stream(&self, input: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.delegate.hash_stream(input)
    }
}

pub fn processor(trace: Trace) -> Arc<DocumentProcessor> {
    let mut ports = crate::crypto::processor_ports();
    ports.cipher = Box::new(Cipher {
        delegate: ports.cipher,
        trace: trace.clone(),
    });
    ports.hasher = Box::new(Hasher {
        delegate: ports.hasher,
        trace,
    });
    Arc::new(
        DocumentProcessor::new(
            ports,
            crate::crypto::evidence_material(),
            Zeroizing::new(vec![0x44; 32]),
        )
        .unwrap(),
    )
}
