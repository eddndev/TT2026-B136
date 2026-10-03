use domain::{crypto::Signature, owner_certificates::Uuid};

use super::{
    evidence::CERTIFICATE_LIMIT, OwnerBindingReceipt, OwnerCertificateError as Error,
    OwnerCertificateService,
};
use crate::ApplicationError;

/// Untrusted public material for one exact signed registration intention.
///
/// The statement must contain the 150 canonical registration bytes. Certificate
/// bytes must be the exact inspected DER returned by preparation. No field is a
/// successful verification, a trusted capture or an authentication capability.
pub struct OwnerRegistrationSubmission {
    pub statement: Vec<u8>,
    pub certificate_der: Vec<u8>,
    pub signature: Signature,
}

impl OwnerCertificateService {
    /// Reconciles exact history before reconstructing a new current preparation.
    ///
    /// Current account and published trust determine new registrations. Their
    /// canonical bytes must equal the submitted expectation; stale intentions
    /// are never silently replaced. No failed commit is retried here.
    pub fn submit_registration(
        &self,
        token: &str,
        binding: Uuid,
        input: OwnerRegistrationSubmission,
    ) -> Result<OwnerBindingReceipt, ApplicationError> {
        let principal = self.owner(token)?;
        if binding.is_nil()
            || input.statement.len() != 150
            || input.certificate_der.is_empty()
            || input.certificate_der.len() > CERTIFICATE_LIMIT
            || input.signature.as_bytes().len() != 384
        {
            return Err(Error::InvalidInput.into());
        }

        if let Some(receipt) = self.ports.store.find(principal.id, binding)? {
            self.validate_receipt(&receipt, principal.id, binding)?;
            if input.statement.as_slice()
                != receipt.record.registration().canonical_bytes().as_slice()
                || input.certificate_der != receipt.check.certificate.der
                || input.signature != receipt.check.signature
            {
                return Err(Error::BindingConflict.into());
            }
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }

        self.reauthenticate(token, &principal)?;
        let prepared = self.prepare_registration(token, binding, &input.certificate_der)?;
        // A nested operation's own authentication cannot replace the caller's
        // full Principal, even when the durable account changed consistently.
        if prepared.account().principal != principal {
            return Err(ApplicationError::InvalidSession);
        }
        self.reauthenticate(token, &principal)?;
        if input.statement.as_slice() != prepared.statement().canonical_bytes().as_slice()
            || input.certificate_der != prepared.certificate().der
        {
            return Err(Error::BindingConflict.into());
        }
        let receipt = self.register(token, &prepared, &input.signature)?;
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}
