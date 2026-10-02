//! Envelope protection for private report captures and artifacts.
//!
//! The authenticated context is the ASCII domain `tt-case-report-envelope-v1`
//! followed by one zero byte, the report UUID's 16 raw bytes, one payload tag
//! (1 = snapshot, 2 = PDF, 3 = CSV), and the 32 raw SHA-256 bytes of the plaintext.
//! All fields have fixed lengths. Changing this encoding requires an explicit
//! envelope version and preservation of the decoder for stored reports.

use application::{case_reports::*, ApplicationError};
use domain::crypto::{
    cipher::SEALED_PAYLOAD_MIN_LEN,
    keys::{DATA_KEY_LEN, KEY_ENCRYPTION_KEY_LEN},
    AuthenticatedCipher, DocumentHasher, KeyManager,
};
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::RingSha256Hasher;

const AAD_DOMAIN: &[u8] = b"tt-case-report-envelope-v1\0";
const AAD_LENGTH: usize = AAD_DOMAIN.len() + 16 + 1 + 32;
const WRAPPED_KEY_LENGTH: usize = DATA_KEY_LEN + SEALED_PAYLOAD_MIN_LEN;

pub struct CaseReportEnvelopeProtector {
    cipher: Arc<dyn AuthenticatedCipher + Send + Sync>,
    keys: Arc<dyn KeyManager + Send + Sync>,
    kek: Zeroizing<Vec<u8>>,
}

impl CaseReportEnvelopeProtector {
    pub fn new(
        cipher: Arc<dyn AuthenticatedCipher + Send + Sync>,
        keys: Arc<dyn KeyManager + Send + Sync>,
        kek: Vec<u8>,
    ) -> Result<Self, ApplicationError> {
        let kek = Zeroizing::new(kek);
        if kek.len() != KEY_ENCRYPTION_KEY_LEN {
            return Err(ApplicationError::InvalidConfiguration(format!(
                "report kek must be {KEY_ENCRYPTION_KEY_LEN} bytes, got {}",
                kek.len()
            )));
        }
        Ok(Self { cipher, keys, kek })
    }
}

impl CaseReportProtector for CaseReportEnvelopeProtector {
    fn seal(
        &self,
        context: CaseReportProtectionContext,
        plaintext: &[u8],
    ) -> Result<ProtectedCaseReportPayload, ApplicationError> {
        validate_identity(context)?;
        validate_length(plaintext.len(), maximum(context.kind))?;
        validate_digest(context, plaintext)?;
        let dek = Zeroizing::new(self.keys.generate_dek()?);
        let wrapped_dek = self.keys.wrap_dek(&self.kek, &dek)?;
        let payload = self.cipher.seal(&dek, &aad(context), plaintext)?;
        Ok(ProtectedCaseReportPayload {
            wrapped_dek,
            payload,
        })
    }

    fn open(
        &self,
        context: CaseReportProtectionContext,
        protected: &ProtectedCaseReportPayload,
    ) -> Result<Vec<u8>, ApplicationError> {
        validate_identity(context)?;
        validate_length(
            protected.payload.as_bytes().len(),
            maximum(context.kind) + SEALED_PAYLOAD_MIN_LEN,
        )?;
        if protected.wrapped_dek.as_bytes().len() != WRAPPED_KEY_LENGTH {
            return Err(inconsistent("wrapped report key has an invalid length"));
        }
        let dek = Zeroizing::new(self.keys.unwrap_dek(&self.kek, &protected.wrapped_dek)?);
        let mut plaintext =
            Zeroizing::new(self.cipher.open(&dek, &aad(context), &protected.payload)?);
        validate_length(plaintext.len(), maximum(context.kind))?;
        validate_digest(context, &plaintext)?;
        Ok(std::mem::take(&mut *plaintext))
    }
}

fn validate_identity(context: CaseReportProtectionContext) -> Result<(), ApplicationError> {
    if context.report_id.as_uuid().is_nil() {
        return Err(inconsistent("report identity must not be nil"));
    }
    Ok(())
}

fn validate_length(actual: usize, maximum: usize) -> Result<(), ApplicationError> {
    if actual > maximum {
        return Err(CaseReportError::CapacityExceeded.into());
    }
    Ok(())
}

fn validate_digest(
    context: CaseReportProtectionContext,
    plaintext: &[u8],
) -> Result<(), ApplicationError> {
    if RingSha256Hasher.hash_bytes(plaintext) != context.plaintext_digest {
        return Err(inconsistent(
            "report plaintext does not match its recorded digest",
        ));
    }
    Ok(())
}

fn maximum(kind: CaseReportPayloadKind) -> usize {
    match kind {
        CaseReportPayloadKind::Snapshot => MAX_REPORT_SNAPSHOT_BYTES,
        CaseReportPayloadKind::Pdf | CaseReportPayloadKind::Csv => MAX_REPORT_ARTIFACT_BYTES,
    }
}

fn aad(context: CaseReportProtectionContext) -> [u8; AAD_LENGTH] {
    let mut bytes = [0; AAD_LENGTH];
    let prefix = AAD_DOMAIN.len();
    bytes[..prefix].copy_from_slice(AAD_DOMAIN);
    bytes[prefix..prefix + 16].copy_from_slice(context.report_id.as_uuid().as_bytes());
    bytes[prefix + 16] = match context.kind {
        CaseReportPayloadKind::Snapshot => 1,
        CaseReportPayloadKind::Pdf => 2,
        CaseReportPayloadKind::Csv => 3,
    };
    bytes[prefix + 17..].copy_from_slice(context.plaintext_digest.as_bytes());
    bytes
}

fn inconsistent(message: &str) -> ApplicationError {
    CaseReportError::StoredInconsistent(message.into()).into()
}
