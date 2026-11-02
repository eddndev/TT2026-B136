//! Party statements of bilateral contracts under the internal declaration
//! profile with mandatory CRL checking, and the current revocation status of
//! a counterparty certificate. Rationale:
//! docs/adr/0086-bilateral-contract-signing.md.

use application::{
    contracts::ContractSignatureVerifier, credential_trust::CredentialTrustSnapshot,
    document_signing::SigningWorkflowError,
};
use domain::{
    crypto::{CredentialCheck, CredentialFailure, DocumentHasher, Signature},
    document_signing::contract::ContractPartyStatement,
};

use super::{
    document_signing::InternalRsaDocumentSigningVerifier,
    internal_profile::{Profile, Verifier},
};
use crate::RingSha256Hasher;

impl ContractSignatureVerifier for InternalRsaDocumentSigningVerifier {
    fn verify_statement(
        &self,
        statement: &ContractPartyStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, SigningWorkflowError> {
        let digest = RingSha256Hasher.hash_bytes(&statement.canonical_bytes());
        let check = Verifier::new(Profile::Declaration).verify(
            &digest,
            certificate,
            signature,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            at,
        )?;
        let material = statement.material();
        if check.trust != trust.inspection
            || material.deployment() != trust.deployment_id
            || material.trust_revision() != trust.revision.get()
            || material.root() != check.trust.root_fingerprint
            || material.leaf() != check.certificate.fingerprint
        {
            return Err(SigningWorkflowError::Inconsistent);
        }
        Ok(check)
    }

    fn certificate_status(
        &self,
        certificate: &[u8],
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<(), CredentialFailure> {
        Verifier::new(Profile::Declaration).revocation(
            certificate,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            at,
        )
    }
}
