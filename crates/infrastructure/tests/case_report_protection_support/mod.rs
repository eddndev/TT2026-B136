use application::case_reports::{CaseReportId, CaseReportPayloadKind, CaseReportProtectionContext};
use domain::{
    crypto::{AuthenticatedCipher, DocumentHasher, KeyManager, SealedPayload, WrappedDek},
    DomainError,
};
use infrastructure::{
    CaseReportEnvelopeProtector, EnvelopeKeyManager, RingAesGcmCipher, RingSha256Hasher,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

pub const KEK: [u8; 32] = [0x52; 32];

pub fn context(kind: CaseReportPayloadKind, bytes: &[u8]) -> CaseReportProtectionContext {
    CaseReportProtectionContext {
        report_id: CaseReportId::new(),
        kind,
        plaintext_digest: RingSha256Hasher.hash_bytes(bytes),
    }
}

pub fn protector() -> CaseReportEnvelopeProtector {
    CaseReportEnvelopeProtector::new(
        Arc::new(RingAesGcmCipher::new()),
        Arc::new(EnvelopeKeyManager::new()),
        KEK.to_vec(),
    )
    .unwrap()
}

pub fn counted(alter_opened_bytes: bool) -> (CaseReportEnvelopeProtector, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let cipher = CountedCipher {
        calls: calls.clone(),
        alter_opened_bytes,
    };
    let keys = CountedKeys(calls.clone());
    let protector =
        CaseReportEnvelopeProtector::new(Arc::new(cipher), Arc::new(keys), KEK.to_vec()).unwrap();
    (protector, calls)
}

struct CountedCipher {
    calls: Arc<AtomicUsize>,
    alter_opened_bytes: bool,
}
impl AuthenticatedCipher for CountedCipher {
    fn seal(&self, key: &[u8], aad: &[u8], plaintext: &[u8]) -> Result<SealedPayload, DomainError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        RingAesGcmCipher::new().seal(key, aad, plaintext)
    }
    fn open(
        &self,
        key: &[u8],
        aad: &[u8],
        payload: &SealedPayload,
    ) -> Result<Vec<u8>, DomainError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut bytes = RingAesGcmCipher::new().open(key, aad, payload)?;
        if self.alter_opened_bytes {
            bytes[0] ^= 1;
        }
        Ok(bytes)
    }
}

struct CountedKeys(Arc<AtomicUsize>);
impl KeyManager for CountedKeys {
    fn generate_dek(&self) -> Result<Vec<u8>, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        EnvelopeKeyManager::new().generate_dek()
    }
    fn wrap_dek(&self, kek: &[u8], dek: &[u8]) -> Result<WrappedDek, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        EnvelopeKeyManager::new().wrap_dek(kek, dek)
    }
    fn unwrap_dek(&self, kek: &[u8], wrapped: &WrappedDek) -> Result<Vec<u8>, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        EnvelopeKeyManager::new().unwrap_dek(kek, wrapped)
    }
    fn rewrap_dek(
        &self,
        old_kek: &[u8],
        new_kek: &[u8],
        wrapped: &WrappedDek,
    ) -> Result<WrappedDek, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        EnvelopeKeyManager::new().rewrap_dek(old_kek, new_kek, wrapped)
    }
}
