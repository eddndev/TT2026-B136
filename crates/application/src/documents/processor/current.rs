use domain::{
    crypto::{CredentialCheck, CredentialFailure as Failure, Signature},
    document_signing::DocumentConsent,
};

use super::DocumentProcessor;
use crate::{credential_trust::CredentialTrustSnapshot, documents::DocumentRecord};

impl DocumentProcessor {
    /// Evaluates the original signed consent under the current public root and CRL.
    /// A newer trust revision does not change the bytes the signer consented to.
    pub fn verify_individual_current(
        &self,
        record: &DocumentRecord,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, Failure> {
        self.validate_record(record)
            .map_err(|_| Failure::InvalidSignature)?;
        let verifier = self
            .individual_verifier
            .as_ref()
            .ok_or(Failure::UnsupportedCertificate)?;
        let evidence = record.evidence.as_ref().ok_or(Failure::InvalidSignature)?;
        let individual = evidence
            .individual
            .as_ref()
            .ok_or(Failure::UnsupportedCertificate)?;
        let consent = DocumentConsent::from_canonical_bytes(&individual.consent_statement)
            .map_err(|_| Failure::InvalidSignature)?;
        let material = consent.material();
        if material.deployment() != trust.deployment_id
            || material.root() != trust.inspection.root_fingerprint
        {
            return Err(Failure::UntrustedIssuer);
        }
        if trust.revision.get() < material.trust_revision()
            || trust.published_at.unix_timestamp() > at
        {
            return Err(Failure::UntrustedCrl);
        }
        let cert = verifier.inspect_certificate(&evidence.signer_certificate_pem)?;
        let inspected =
            verifier.inspect_trust(&trust.inspection.root_der, &trust.inspection.crl_der, at)?;
        let hasher = self.ports.hasher.as_ref();
        if cert.fingerprint != material.leaf() || cert.fingerprint != hasher.hash_bytes(&cert.der) {
            return Err(Failure::MalformedCertificate);
        }
        if inspected != trust.inspection
            || inspected.root_fingerprint != hasher.hash_bytes(&inspected.root_der)
            || inspected.crl_digest != hasher.hash_bytes(&inspected.crl_der)
        {
            return Err(Failure::UntrustedCrl);
        }
        let digest = hasher.hash_bytes(&individual.consent_statement);
        let signature = Signature::from_bytes(individual.consent_signature.clone())
            .map_err(|_| Failure::InvalidSignature)?;
        let check = verifier.verify(
            &digest,
            &cert.der,
            &signature,
            &inspected.root_der,
            &inspected.crl_der,
            at,
        )?;
        if check.certificate != cert
            || check.trust != inspected
            || check.statement_digest != digest
            || check.signature != signature
            || check.checked_at != at
            || check.valid_from != cert.summary.not_before_unix.max(inspected.valid_from)
            || check.valid_until != cert.summary.not_after_unix.min(inspected.valid_until)
            || at < check.valid_from
            || at > check.valid_until
            || at < inspected.crl_this_update
            || at > inspected.crl_next_update
        {
            return Err(Failure::InvalidSignature);
        }
        Ok(check)
    }
}
