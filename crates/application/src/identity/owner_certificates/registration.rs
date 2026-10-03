use domain::{
    crypto::Signature,
    owner_certificates::{BindingMaterial, BindingStatement, Uuid},
};

use super::{
    evidence::CERTIFICATE_LIMIT, OwnerBindingCommit, OwnerBindingReceipt,
    OwnerCertificateError as Error, OwnerCertificateService, PreparedOwnerRegistration,
    VerifiedOwnerRegistration,
};
use crate::ApplicationError;

impl OwnerCertificateService {
    pub fn prepare_registration(
        &self,
        token: &str,
        binding: Uuid,
        certificate: &[u8],
    ) -> Result<PreparedOwnerRegistration, ApplicationError> {
        let principal = self.owner(token)?;
        if binding.is_nil() || certificate.is_empty() || certificate.len() > CERTIFICATE_LIMIT {
            return Err(Error::InvalidInput.into());
        }
        let certificate = self
            .ports
            .verifier
            .inspect_certificate(certificate)
            .map_err(Error::Credential)?;
        self.validate_certificate(&certificate)?;
        let context = self.ports.store.load_registration(principal.id)?;
        let owner = self.account(&context.account, &principal)?;
        let trust = context.trust.ok_or(Error::TrustUnavailable)?;
        self.validate_trust(&trust)?;
        let material = BindingMaterial::new(
            trust.deployment_id,
            binding,
            trust.inspection.root_fingerprint,
            certificate.fingerprint,
            trust.revision.get(),
        )
        .map_err(|_| Error::Inconsistent)?;
        let statement = BindingStatement::new(owner, principal.id, material)
            .map_err(|_| Error::Inconsistent)?;
        self.reauthenticate(token, &principal)?;
        Ok(PreparedOwnerRegistration {
            account: context.account,
            statement,
            certificate,
            trust,
        })
    }

    pub fn register(
        &self,
        token: &str,
        prepared: &PreparedOwnerRegistration,
        signature: &Signature,
    ) -> Result<OwnerBindingReceipt, ApplicationError> {
        let principal = self.owner(token)?;
        self.account(prepared.account(), &principal)?;
        if signature.as_bytes().len() != 384 {
            return Err(Error::InvalidInput.into());
        }
        let binding = prepared.statement().material().binding();
        if let Some(receipt) = self.ports.store.find(principal.id, binding)? {
            self.same_intent(&receipt, prepared, signature)?;
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let context = self.ports.store.load_registration(principal.id)?;
        self.account(&context.account, &principal)?;
        if context.account != *prepared.account() {
            return Err(Error::AccountChanged.into());
        }
        if context.trust.as_ref() != Some(prepared.trust()) {
            return Err(Error::TrustChanged.into());
        }
        let started = self.ports.clock.now();
        let check = self.ports.verifier.verify_registration(
            prepared.statement(),
            &prepared.certificate().der,
            signature,
            prepared.trust(),
            started.unix_timestamp(),
        )?;
        self.validate_check(prepared.statement(), &check, prepared.trust())?;
        if check.certificate != *prepared.certificate()
            || check.signature != *signature
            || check.checked_at != started.unix_timestamp()
        {
            return Err(Error::Inconsistent.into());
        }
        let not_before = self.ports.clock.now();
        if not_before < started {
            return Err(Error::Inconsistent.into());
        }
        self.check_time(&check, not_before.unix_timestamp())?;
        self.reauthenticate(token, &principal)?;
        let verified = VerifiedOwnerRegistration {
            registration: prepared.clone(),
            check: check.clone(),
            not_before,
        };
        match self.ports.store.commit_registration(verified)? {
            OwnerBindingCommit::Applied(receipt) => {
                self.same_intent(&receipt, prepared, signature)?;
                if receipt.check != check
                    || receipt.registered_at < not_before
                    || receipt.withdrawn_at.is_some()
                {
                    return Err(Error::Inconsistent.into());
                }
                Ok(receipt)
            }
            OwnerBindingCommit::Existing(receipt) => {
                self.same_intent(&receipt, prepared, signature)?;
                Ok(receipt)
            }
        }
    }
}
