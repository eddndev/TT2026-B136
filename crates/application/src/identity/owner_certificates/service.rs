use domain::{
    identity::Role,
    owner_certificates::{OwnerAccount, Uuid},
};

use super::{
    OwnerBindingAccount, OwnerBindingReceipt, OwnerCertificateError, OwnerCertificatePorts,
};
use crate::{identity::Principal, ApplicationError};

pub struct OwnerCertificateService {
    pub(super) ports: OwnerCertificatePorts,
}

impl OwnerCertificateService {
    pub fn new(ports: OwnerCertificatePorts) -> Self {
        Self { ports }
    }

    pub(super) fn owner(&self, token: &str) -> Result<Principal, ApplicationError> {
        let principal = self.ports.identity.authenticate(token)?;
        if principal.role != Role::Owner {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(principal)
    }

    pub(super) fn reauthenticate(
        &self,
        token: &str,
        principal: &Principal,
    ) -> Result<(), ApplicationError> {
        if self.ports.identity.authenticate(token)? != *principal {
            return Err(ApplicationError::InvalidSession);
        }
        Ok(())
    }

    pub(super) fn account(
        &self,
        account: &OwnerBindingAccount,
        principal: &Principal,
    ) -> Result<OwnerAccount, ApplicationError> {
        if account.principal != *principal {
            return Err(OwnerCertificateError::AccountChanged.into());
        }
        OwnerAccount::new(
            principal.id,
            principal.role,
            account.active,
            account.revision,
            account.auth_generation,
        )
        .map_err(|_| OwnerCertificateError::AccountChanged.into())
    }

    /// Historical receipts never require currently valid certificate material.
    pub fn receipt(
        &self,
        token: &str,
        binding: Uuid,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        let principal = self.owner(token)?;
        if binding.is_nil() {
            return Err(OwnerCertificateError::InvalidInput.into());
        }
        let receipt = self.ports.store.find(principal.id, binding)?;
        if let Some(value) = &receipt {
            self.validate_receipt(value, principal.id, binding)?;
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }

    /// Current means unwithdrawn, not currently valid certificate or trust material.
    pub fn current_receipt(
        &self,
        token: &str,
    ) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        let principal = self.owner(token)?;
        let receipt = self.ports.store.find_current(principal.id)?;
        if let Some(value) = &receipt {
            let binding = value.record.registration().material().binding();
            self.validate_receipt(value, principal.id, binding)?;
            if value.record.withdrawal().is_some() {
                return Err(OwnerCertificateError::Inconsistent.into());
            }
        }
        self.reauthenticate(token, &principal)?;
        Ok(receipt)
    }
}
