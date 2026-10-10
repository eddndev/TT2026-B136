use domain::{
    cases::CaseId, crypto::DocumentVersionRef, document_signing::DocumentConsent,
    identity::Permission,
};

use super::{
    CurrentSigningStatus, DocumentSigningService, SigningVerification,
    SigningWorkflowError as Error,
};
use crate::ApplicationError;

impl DocumentSigningService {
    /// Historical evidence and current certificate status have independent outcomes.
    pub fn verify(
        &self,
        token: &str,
        case: CaseId,
        reference: DocumentVersionRef,
    ) -> Result<SigningVerification, ApplicationError> {
        let principal = self.ports.identity.authenticate(token)?;
        if !principal.role.allows(Permission::VerifyDocument) {
            return Err(ApplicationError::PermissionDenied);
        }
        if case.as_uuid().is_nil() || reference.id.as_uuid().is_nil() {
            return Err(Error::Input.into());
        }
        let context = self
            .ports
            .store
            .verification_context(&principal, case, reference)?;
        let record = &context.record;
        if record.id != reference.id || record.version != reference.version {
            return Err(Error::Inconsistent.into());
        }
        self.ports.processor.validate_record(record)?;
        let evidence = record
            .evidence
            .as_ref()
            .ok_or_else(|| ApplicationError::DocumentNotSealed(record.id.to_string()))?;
        let checked_at = self.ports.clock.now();
        let (signing, evaluated_at, certificate_current, withdrawn) = match &evidence.individual {
            None => {
                if context.signing.is_some() {
                    return Err(Error::Inconsistent.into());
                }
                (
                    None,
                    checked_at.unix_timestamp(),
                    CurrentSigningStatus::NotIndividual,
                    None,
                )
            }
            Some(individual) => {
                let consent = DocumentConsent::from_canonical_bytes(&individual.consent_statement)
                    .map_err(|_| Error::Inconsistent)?;
                if consent.case() != case || consent.document() != reference {
                    return Err(Error::Inconsistent.into());
                }
                let receipt = context.signing.ok_or(Error::Inconsistent)?;
                if receipt.consent != consent
                    || receipt.evidence != *evidence
                    || receipt.accepted_at.unix_timestamp() != individual.accepted_at
                {
                    return Err(Error::Inconsistent.into());
                }
                let current = match context.trust.as_ref() {
                    None => CurrentSigningStatus::TrustUnavailable,
                    Some(trust) => match self.ports.processor.verify_individual_current(
                        record,
                        trust,
                        checked_at.unix_timestamp(),
                    ) {
                        Ok(_) => CurrentSigningStatus::Valid,
                        Err(reason) => CurrentSigningStatus::Rejected(reason),
                    },
                };
                (
                    Some(receipt),
                    individual.accepted_at,
                    current,
                    context.binding_withdrawn_at,
                )
            }
        };
        let historical = self.ports.processor.verify(record, evaluated_at)?;
        if self.ports.identity.authenticate(token)? != principal {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(SigningVerification {
            historical,
            certificate_current,
            checked_at,
            binding_withdrawn_at: withdrawn,
            signing,
        })
    }
}
