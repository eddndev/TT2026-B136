use super::{SigningBindingReceipt, SigningWorkflowError as Error};
use crate::{credential_trust::CredentialTrustSnapshot, ApplicationError};
use domain::{
    crypto::{CredentialCertificate, CredentialCheck, CredentialFailure, DocumentHasher},
    document_signing::{SigningBindingStatement, Uuid},
    identity::UserId,
};

pub(super) const CERTIFICATE_LIMIT: usize = 16 * 1024;
const CRL_LIMIT: usize = 1024 * 1024;

pub(super) fn certificate(
    hasher: &dyn DocumentHasher,
    value: &CredentialCertificate,
) -> Result<(), ApplicationError> {
    if value.der.is_empty()
        || value.der.len() > CERTIFICATE_LIMIT
        || hasher.hash_bytes(&value.der) != value.fingerprint
        || value.summary.not_before_unix > value.summary.not_after_unix
    {
        return Err(Error::Inconsistent.into());
    }
    Ok(())
}
pub(super) fn trust(
    hasher: &dyn DocumentHasher,
    value: &CredentialTrustSnapshot,
) -> Result<(), ApplicationError> {
    let inspection = &value.inspection;
    if value.deployment_id.is_nil()
        || inspection.root_der.is_empty()
        || inspection.root_der.len() > CERTIFICATE_LIMIT
        || inspection.crl_der.is_empty()
        || inspection.crl_der.len() > CRL_LIMIT
        || hasher.hash_bytes(&inspection.root_der) != inspection.root_fingerprint
        || hasher.hash_bytes(&inspection.crl_der) != inspection.crl_digest
        || inspection.crl_this_update >= inspection.crl_next_update
        || inspection.valid_from > inspection.valid_until
        || inspection.valid_from < inspection.crl_this_update
        || inspection.valid_until > inspection.crl_next_update
    {
        return Err(Error::Inconsistent.into());
    }
    Ok(())
}
pub(super) fn check(
    hasher: &dyn DocumentHasher,
    statement: &SigningBindingStatement,
    check: &CredentialCheck,
    snapshot: &CredentialTrustSnapshot,
) -> Result<(), ApplicationError> {
    certificate(hasher, &check.certificate)?;
    trust(hasher, snapshot)?;
    let material = statement.material();
    let summary = &check.certificate.summary;
    if material.deployment() != snapshot.deployment_id
        || material.trust_revision() != snapshot.revision.get()
        || material.root() != snapshot.inspection.root_fingerprint
        || material.leaf() != check.certificate.fingerprint
        || check.trust != snapshot.inspection
        || check.statement_digest != hasher.hash_bytes(&statement.canonical_bytes())
        || check.signature.as_bytes().len() != 384
        || check.valid_from != summary.not_before_unix.max(snapshot.inspection.valid_from)
        || check.valid_until != summary.not_after_unix.min(snapshot.inspection.valid_until)
        || check.valid_from > check.valid_until
        || check.checked_at < check.valid_from
        || check.checked_at > check.valid_until
    {
        return Err(Error::Inconsistent.into());
    }
    Ok(())
}

/// Validates captured shape, digests, identity and time without claiming cryptography.
/// Historical checks use their captured trust, never today's mutable trust or clock.
pub fn validate_binding_receipt(
    hasher: &dyn DocumentHasher,
    value: &SigningBindingReceipt,
    actor: UserId,
    binding: Uuid,
) -> Result<(), ApplicationError> {
    if value.statement.account().id() != actor || value.statement.material().binding() != binding {
        return Err(Error::Inconsistent.into());
    }
    check(hasher, &value.statement, &value.check, &value.trust)?;
    let at = value.registered_at.unix_timestamp();
    if at < value.check.checked_at
        || at > value.check.valid_until
        || value.registered_at < value.trust.published_at
        || value
            .withdrawn_at
            .is_some_and(|at| at < value.registered_at)
    {
        return Err(Error::Inconsistent.into());
    }
    Ok(())
}

pub(super) fn check_time(check: &CredentialCheck, at: i64) -> Result<(), ApplicationError> {
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
