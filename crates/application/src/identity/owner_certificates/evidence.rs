use domain::{
    crypto::{CredentialCertificate, CredentialCheck, CredentialFailure, Signature},
    identity::UserId,
    owner_certificates::{BindingStatement, Uuid},
};

use super::{
    OwnerBindingReceipt, OwnerCertificateError as Error, OwnerCertificateService,
    PreparedOwnerRegistration,
};
use crate::{credential_trust::CredentialTrustSnapshot, ApplicationError};

pub(super) const CERTIFICATE_LIMIT: usize = 16 * 1024;
const CRL_LIMIT: usize = 1024 * 1024;

impl OwnerCertificateService {
    pub(super) fn validate_certificate(
        &self,
        value: &CredentialCertificate,
    ) -> Result<(), ApplicationError> {
        let summary = &value.summary;
        if value.der.is_empty()
            || value.der.len() > CERTIFICATE_LIMIT
            || self.ports.hasher.hash_bytes(&value.der) != value.fingerprint
            || summary.not_before_unix > summary.not_after_unix
        {
            return Err(Error::Inconsistent.into());
        }
        Ok(())
    }

    pub(super) fn validate_trust(
        &self,
        value: &CredentialTrustSnapshot,
    ) -> Result<(), ApplicationError> {
        let trust = &value.inspection;
        if value.deployment_id.is_nil()
            || trust.root_der.is_empty()
            || trust.root_der.len() > CERTIFICATE_LIMIT
            || trust.crl_der.is_empty()
            || trust.crl_der.len() > CRL_LIMIT
            || self.ports.hasher.hash_bytes(&trust.root_der) != trust.root_fingerprint
            || self.ports.hasher.hash_bytes(&trust.crl_der) != trust.crl_digest
            || trust.crl_this_update >= trust.crl_next_update
            || trust.valid_from > trust.valid_until
            || trust.valid_from < trust.crl_this_update
            || trust.valid_until > trust.crl_next_update
        {
            return Err(Error::Inconsistent.into());
        }
        Ok(())
    }

    pub(super) fn validate_check(
        &self,
        statement: &BindingStatement,
        check: &CredentialCheck,
        trust: &CredentialTrustSnapshot,
    ) -> Result<(), ApplicationError> {
        self.validate_certificate(&check.certificate)?;
        self.validate_trust(trust)?;
        let material = statement.material();
        let summary = &check.certificate.summary;
        if material.deployment() != trust.deployment_id
            || material.trust_revision() != trust.revision.get()
            || material.root() != trust.inspection.root_fingerprint
            || material.leaf() != check.certificate.fingerprint
            || check.trust != trust.inspection
            || check.statement_digest != self.ports.hasher.hash_bytes(&statement.canonical_bytes())
            || check.signature.as_bytes().len() != 384
            || check.valid_from != summary.not_before_unix.max(trust.inspection.valid_from)
            || check.valid_until != summary.not_after_unix.min(trust.inspection.valid_until)
            || check.valid_from > check.valid_until
            || check.checked_at < check.valid_from
            || check.checked_at > check.valid_until
        {
            return Err(Error::Inconsistent.into());
        }
        Ok(())
    }

    pub(super) fn validate_receipt(
        &self,
        value: &OwnerBindingReceipt,
        actor: UserId,
        binding: Uuid,
    ) -> Result<(), ApplicationError> {
        let registration = value.record.registration();
        if value.owner != actor
            || registration.owner().id() != actor
            || registration.material().binding() != binding
        {
            return Err(Error::Inconsistent.into());
        }
        self.validate_check(registration, &value.check, &value.trust)?;
        let registered = value.registered_at.unix_timestamp();
        if registered < value.check.checked_at
            || registered > value.check.valid_until
            || value.registered_at < value.trust.published_at
        {
            return Err(Error::Inconsistent.into());
        }
        match (value.record.withdrawal(), value.withdrawn_at) {
            (None, None) => {}
            (Some(_), Some(at)) if at >= value.registered_at => {}
            _ => return Err(Error::Inconsistent.into()),
        }
        Ok(())
    }

    pub(super) fn same_intent(
        &self,
        receipt: &OwnerBindingReceipt,
        prepared: &PreparedOwnerRegistration,
        signature: &Signature,
    ) -> Result<(), ApplicationError> {
        // Compare exact public bytes before trusting any derived digest or summary.
        if receipt.record.registration() != prepared.statement()
            || receipt.check.certificate != *prepared.certificate()
            || receipt.check.signature != *signature
            || receipt.trust != *prepared.trust()
        {
            return Err(Error::BindingConflict.into());
        }
        self.validate_receipt(
            receipt,
            prepared.account().principal.id,
            prepared.statement().material().binding(),
        )
    }

    pub(super) fn check_time(
        &self,
        check: &CredentialCheck,
        at: i64,
    ) -> Result<(), ApplicationError> {
        if at < check.checked_at {
            return Err(Error::Inconsistent.into());
        }
        if at < check.valid_from {
            return Err(Error::Credential(CredentialFailure::NotYetValid).into());
        }
        if at > check.valid_until {
            return Err(Error::Credential(CredentialFailure::Expired).into());
        }
        Ok(())
    }
}

pub(super) fn same_registration(left: &OwnerBindingReceipt, right: &OwnerBindingReceipt) -> bool {
    left.owner == right.owner
        && left.record.registration() == right.record.registration()
        && left.check == right.check
        && left.trust == right.trust
        && left.registered_at == right.registered_at
}
