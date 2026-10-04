use domain::{identity::UserId, owner_certificates::Uuid};

use super::IdentityService;
use crate::{
    identity::{
        certificate_login::{
            validation, CertificateLoginContext, CertificateLoginPorts,
            CertificateSessionProvenance, StoredCertificateLogin,
        },
        Principal,
    },
    ApplicationError,
};

impl IdentityService {
    pub(super) fn certificate_ports(&self) -> Result<&CertificateLoginPorts, ApplicationError> {
        self.certificate_login
            .as_ref()
            .ok_or(ApplicationError::InvalidCredentials)
    }

    pub(super) fn certificate_context(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<CertificateLoginContext, ApplicationError> {
        let ports = self.certificate_ports()?;
        let context = ports
            .authority
            .load(owner, binding)?
            .filter(|v| v.account.principal.id == owner && v.binding_id == binding)
            .ok_or(ApplicationError::InvalidCredentials)?;
        validation::provenance(&context, ports.hasher.as_ref(), self.now_seconds())?;
        let user = self
            .ports
            .users
            .find_by_id(owner)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        if !user.active
            || context.account.principal != Principal::from(&user)
            || context.account.auth_generation != user.auth_generation
            || context.account.revision != user.revision
        {
            return Err(ApplicationError::InvalidCredentials);
        }
        // Re-read authority after the user lookup rather than admitting its stale snapshot.
        let confirmed = ports
            .authority
            .load(owner, binding)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        if confirmed != context {
            return Err(ApplicationError::InvalidCredentials);
        }
        validation::provenance(&confirmed, ports.hasher.as_ref(), self.now_seconds())?;
        Ok(confirmed)
    }

    pub(super) fn confirm_certificate_capture(
        &self,
        capture: &StoredCertificateLogin,
    ) -> Result<CertificateSessionProvenance, ApplicationError> {
        let context = self.certificate_context(
            capture.context.account.principal.id,
            capture.context.binding_id,
        )?;
        if context != capture.context {
            return Err(ApplicationError::InvalidCredentials);
        }
        validation::capture(
            capture,
            self.certificate_ports()?.hasher.as_ref(),
            self.now_seconds(),
        )
    }

    pub(super) fn guard_certificate(
        &self,
        origin: &CertificateSessionProvenance,
    ) -> Result<(), ApplicationError> {
        let context = self.certificate_context(origin.principal.id, origin.binding_id)?;
        let current = validation::provenance(
            &context,
            self.certificate_ports()?.hasher.as_ref(),
            self.now_seconds(),
        )?;
        if current != *origin {
            return Err(ApplicationError::InvalidCredentials);
        }
        Ok(())
    }

    pub(super) fn now_seconds(&self) -> i64 {
        self.ports.clock.now().unix_timestamp()
    }
}

pub(super) fn mfa_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::InvalidCredentials | ApplicationError::InvalidSession => {
            ApplicationError::MfaRejected
        }
        other => other,
    }
}

pub(super) fn session_error(error: ApplicationError) -> ApplicationError {
    match error {
        ApplicationError::InvalidCredentials => ApplicationError::InvalidSession,
        other => other,
    }
}
