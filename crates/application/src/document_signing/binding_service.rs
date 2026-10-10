use super::{
    evidence, PreparedSigningBinding, SigningAccountContext, SigningBindingPorts,
    SigningBindingReceipt, SigningWorkflowError as Error, VerifiedSigningBinding,
};
use crate::{identity::Principal, ApplicationError};
use domain::{
    crypto::Signature,
    document_signing::{SigningBindingMaterial, SigningBindingStatement, Uuid},
    identity::Role,
};

pub struct SigningBindingService {
    ports: SigningBindingPorts,
}
impl SigningBindingService {
    pub fn new(ports: SigningBindingPorts) -> Self {
        Self { ports }
    }

    fn principal(&self, token: &str) -> Result<Principal, ApplicationError> {
        let principal = self.ports.identity.authenticate(token)?;
        if principal.role != Role::Litigator {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal)
    }
    fn reauthenticate(&self, token: &str, expected: &Principal) -> Result<(), ApplicationError> {
        if self.principal(token)? != *expected {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }
    fn context(&self, principal: &Principal) -> Result<SigningAccountContext, ApplicationError> {
        let context = self.ports.store.load_context(principal.id)?;
        if context.principal != *principal || context.account.id() != principal.id {
            return Err(Error::AccountChanged.into());
        }
        Ok(context)
    }
    fn validate_receipt(
        &self,
        receipt: &SigningBindingReceipt,
        principal: &Principal,
        binding: Uuid,
    ) -> Result<(), ApplicationError> {
        evidence::validate_binding_receipt(
            self.ports.hasher.as_ref(),
            receipt,
            principal.id,
            binding,
        )
    }

    pub fn prepare(
        &self,
        token: &str,
        binding: Uuid,
        certificate: &[u8],
    ) -> Result<PreparedSigningBinding, ApplicationError> {
        let principal = self.principal(token)?;
        let prepared = self.prepare_for(&principal, binding, certificate)?;
        self.reauthenticate(token, &principal)?;
        Ok(prepared)
    }
    fn prepare_for(
        &self,
        principal: &Principal,
        binding: Uuid,
        certificate: &[u8],
    ) -> Result<PreparedSigningBinding, ApplicationError> {
        if binding.is_nil()
            || certificate.is_empty()
            || certificate.len() > evidence::CERTIFICATE_LIMIT
        {
            return Err(Error::Input.into());
        }
        let certificate = self
            .ports
            .verifier
            .inspect_certificate(certificate)
            .map_err(Error::Credential)?;
        evidence::certificate(self.ports.hasher.as_ref(), &certificate)?;
        let context = self.context(principal)?;
        let trust = context.trust.clone().ok_or(Error::TrustUnavailable)?;
        evidence::trust(self.ports.hasher.as_ref(), &trust)?;
        let material = SigningBindingMaterial::new(
            trust.deployment_id,
            binding,
            trust.inspection.root_fingerprint,
            certificate.fingerprint,
            trust.revision.get(),
        )
        .map_err(|_| Error::Inconsistent)?;
        let statement = SigningBindingStatement::new(context.account, principal.id, material)
            .map_err(|_| Error::Inconsistent)?;
        Ok(PreparedSigningBinding {
            context,
            statement,
            certificate,
            trust,
        })
    }

    /// Accepts the canonical DER returned by preparation and an external RSA proof.
    pub fn register(
        &self,
        token: &str,
        statement: &SigningBindingStatement,
        certificate: &[u8],
        signature: &Signature,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        if certificate.is_empty()
            || certificate.len() > evidence::CERTIFICATE_LIMIT
            || signature.as_bytes().len() != 384
        {
            return Err(Error::Input.into());
        }
        if statement.account().id() != principal.id {
            return Err(Error::Conflict.into());
        }
        let binding = statement.material().binding();
        if let Some(receipt) = self.ports.store.find(principal.id, binding)? {
            self.same_intent(&receipt, &principal, statement, certificate, signature)?;
            self.reauthenticate(token, &principal)?;
            return Ok(receipt);
        }
        let prepared = self.prepare_for(&principal, binding, certificate)?;
        if statement.account() != prepared.statement.account() {
            return Err(Error::AccountChanged.into());
        }
        let actual = prepared.statement.material();
        let expected = statement.material();
        if actual.deployment() != expected.deployment()
            || actual.root() != expected.root()
            || actual.trust_revision() != expected.trust_revision()
        {
            return Err(Error::TrustChanged.into());
        }
        if prepared.statement != *statement || prepared.certificate.der != certificate {
            return Err(Error::Conflict.into());
        }
        let started = self.ports.clock.now();
        let check = self.ports.verifier.verify_binding(
            statement,
            certificate,
            signature,
            &prepared.trust,
            started.unix_timestamp(),
        )?;
        evidence::check(
            self.ports.hasher.as_ref(),
            statement,
            &check,
            &prepared.trust,
        )?;
        if check.certificate != prepared.certificate
            || check.signature != *signature
            || check.checked_at != started.unix_timestamp()
        {
            return Err(Error::Inconsistent.into());
        }
        let not_before = self.ports.clock.now();
        if not_before < started {
            return Err(Error::Inconsistent.into());
        }
        evidence::check_time(&check, not_before.unix_timestamp())?;
        self.reauthenticate(token, &principal)?;
        let trust = prepared.trust.clone();
        let receipt = self.ports.store.commit_binding(VerifiedSigningBinding {
            prepared,
            check,
            not_before,
        })?;
        self.same_intent(&receipt, &principal, statement, certificate, signature)?;
        if receipt.trust != trust {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    fn same_intent(
        &self,
        receipt: &SigningBindingReceipt,
        principal: &Principal,
        statement: &SigningBindingStatement,
        certificate: &[u8],
        signature: &Signature,
    ) -> Result<(), ApplicationError> {
        self.validate_receipt(receipt, principal, statement.material().binding())?;
        if receipt.statement != *statement
            || receipt.check.certificate.der != certificate
            || receipt.check.signature != *signature
        {
            return Err(Error::Conflict.into());
        }
        Ok(())
    }

    pub fn current(&self, token: &str) -> Result<Option<SigningBindingReceipt>, ApplicationError> {
        let principal = self.principal(token)?;
        let receipt = self.ports.store.find_current(principal.id)?;
        if let Some(value) = &receipt {
            self.validate_receipt(value, &principal, value.statement.material().binding())?;
            if value.withdrawn_at.is_some() {
                return Err(Error::Inconsistent.into());
            }
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
    pub fn get(
        &self,
        token: &str,
        binding: Uuid,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        if binding.is_nil() {
            return Err(Error::Input.into());
        }
        let receipt = self
            .ports
            .store
            .find(principal.id, binding)?
            .ok_or(Error::NotFound)?;
        self.validate_receipt(&receipt, &principal, binding)?;
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
    pub fn withdraw(
        &self,
        token: &str,
        binding: Uuid,
    ) -> Result<SigningBindingReceipt, ApplicationError> {
        let principal = self.principal(token)?;
        if binding.is_nil() {
            return Err(Error::Input.into());
        }
        let original = self
            .ports
            .store
            .find(principal.id, binding)?
            .ok_or(Error::NotFound)?;
        self.validate_receipt(&original, &principal, binding)?;
        let context = self.context(&principal)?;
        if context.account.revision() < original.statement.account().revision()
            || context.account.generation() < original.statement.account().generation()
        {
            return Err(Error::AccountChanged.into());
        }
        self.reauthenticate(token, &principal)?;
        let receipt = self.ports.store.withdraw(
            &principal,
            binding,
            context.account,
            self.ports.clock.now(),
        )?;
        self.validate_receipt(&receipt, &principal, binding)?;
        if receipt.statement != original.statement
            || receipt.check != original.check
            || receipt.trust != original.trust
            || receipt.registered_at != original.registered_at
            || receipt.withdrawn_at.is_none()
            || original
                .withdrawn_at
                .is_some_and(|at| receipt.withdrawn_at != Some(at))
        {
            return Err(Error::Inconsistent.into());
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}
