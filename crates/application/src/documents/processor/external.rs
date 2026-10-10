use domain::crypto::Signature;

use super::{ensure_signature_matches, ensure_timestamp_matches, DocumentProcessor};
use crate::{
    documents::{DocumentRecord, SealedEvidence},
    ApplicationError,
};

/// Public signer material already admitted by the individual signing workflow.
/// TSA configuration remains the processor's independently configured trust.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalSignerMaterial {
    pub certificate_pem: Vec<u8>,
    pub issuer_pem: Vec<u8>,
    pub crl_pem: Vec<u8>,
}

impl DocumentProcessor {
    /// Verifies an externally produced content signature before requesting a timestamp.
    /// The caller must attach and atomically commit its individual consent evidence.
    pub fn seal_external(
        &self,
        input: &DocumentRecord,
        signature: &Signature,
        signer: &ExternalSignerMaterial,
    ) -> Result<DocumentRecord, ApplicationError> {
        if input.is_sealed() {
            return Err(ApplicationError::DocumentAlreadySealed(
                input.id.to_string(),
            ));
        }
        if signer.certificate_pem.is_empty()
            || signer.issuer_pem.is_empty()
            || signer.crl_pem.is_empty()
        {
            return Err(ApplicationError::InvalidInput(
                "external signer material is incomplete".into(),
            ));
        }
        let _plaintext = self.plaintext(input)?;
        ensure_signature_matches(
            self.ports.signature_verifier.as_ref(),
            &input.digest,
            signature,
            &signer.certificate_pem,
        )?;
        let token = self.ports.timestamp_service.request(&input.digest)?;
        let anchor = self
            .material
            .tsa_chain_pem
            .as_deref()
            .unwrap_or(&self.material.issuer_certificate_pem);
        ensure_timestamp_matches(
            self.ports.timestamp_verifier.as_ref(),
            &token,
            &input.digest,
            anchor,
        )?;
        let mut record = input.clone();
        record.seal(SealedEvidence {
            signature: signature.as_bytes().to_vec(),
            timestamp_token: token,
            signer_certificate_pem: signer.certificate_pem.clone(),
            issuer_certificate_pem: signer.issuer_pem.clone(),
            crl_pem: signer.crl_pem.clone(),
            // A personal issuer need not be the configured timestamp authority.
            tsa_chain_pem: Some(anchor.to_vec()),
            openssl_version: self.material.openssl_version.clone(),
            individual: None,
        })?;
        Ok(record)
    }
}
