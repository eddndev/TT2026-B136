use domain::{
    crypto::{CredentialCheck, DocumentHasher, Signature},
    owner_certificate_login::{LoginAccount, LoginNonce, LoginStatement},
    owner_certificates::BindingMaterial,
};

use super::{CertificateLoginContext, CertificateSessionProvenance, StoredCertificateLogin};
use crate::ApplicationError;

const CERTIFICATE_LIMIT: usize = 16 * 1024;
const CRL_LIMIT: usize = 1024 * 1024;

pub(crate) fn provenance(
    context: &CertificateLoginContext,
    hasher: &dyn DocumentHasher,
    at: i64,
) -> Result<CertificateSessionProvenance, ApplicationError> {
    let account = &context.account;
    LoginAccount::new(
        account.principal.id,
        account.principal.role,
        account.active,
        account.auth_generation,
    )
    .map_err(|_| ApplicationError::InvalidCredentials)?;
    let certificate = &context.certificate;
    let summary = &certificate.summary;
    let trust = &context.trust;
    let inspected = &trust.inspection;
    let valid_from = summary.not_before_unix.max(inspected.valid_from);
    let valid_until = summary.not_after_unix.min(inspected.valid_until);
    if context.binding_id.is_nil()
        || trust.deployment_id.is_nil()
        || account.revision > i64::MAX as u64
        || certificate.der.is_empty()
        || certificate.der.len() > CERTIFICATE_LIMIT
        || hasher.hash_bytes(&certificate.der) != certificate.fingerprint
        || summary.not_before_unix > summary.not_after_unix
        || inspected.root_der.is_empty()
        || inspected.root_der.len() > CERTIFICATE_LIMIT
        || inspected.crl_der.is_empty()
        || inspected.crl_der.len() > CRL_LIMIT
        || hasher.hash_bytes(&inspected.root_der) != inspected.root_fingerprint
        || hasher.hash_bytes(&inspected.crl_der) != inspected.crl_digest
        || inspected.crl_this_update >= inspected.crl_next_update
        || inspected.valid_from < inspected.crl_this_update
        || inspected.valid_until > inspected.crl_next_update
        || inspected.valid_from > inspected.valid_until
        || trust.published_at.unix_timestamp() > at
        || at < 0
        || at < valid_from
        || at >= valid_until
        || valid_until.checked_mul(1000).is_none()
    {
        return Err(ApplicationError::InvalidCredentials);
    }
    Ok(CertificateSessionProvenance {
        principal: account.principal.clone(),
        auth_generation: account.auth_generation,
        binding_id: context.binding_id,
        deployment_id: trust.deployment_id,
        trust_revision: trust.revision.get(),
        root_fingerprint: inspected.root_fingerprint,
        leaf_fingerprint: certificate.fingerprint,
        crl_digest: inspected.crl_digest,
        valid_from_unix_seconds: valid_from,
        valid_until_unix_seconds: valid_until,
    })
}

pub(crate) fn statement(
    context: &CertificateLoginContext,
    nonce: LoginNonce,
    issued: i64,
    expires: i64,
) -> Result<LoginStatement, ApplicationError> {
    let account = &context.account;
    let owner = LoginAccount::new(
        account.principal.id,
        account.principal.role,
        account.active,
        account.auth_generation,
    )
    .map_err(|_| ApplicationError::InvalidCredentials)?;
    let material = BindingMaterial::new(
        context.trust.deployment_id,
        context.binding_id,
        context.trust.inspection.root_fingerprint,
        context.certificate.fingerprint,
        context.trust.revision.get(),
    )
    .map_err(|_| ApplicationError::InvalidCredentials)?;
    LoginStatement::new(owner, material, nonce, issued, expires)
        .map_err(|_| ApplicationError::InvalidCredentials)
}

pub(crate) fn capture(
    value: &StoredCertificateLogin,
    hasher: &dyn DocumentHasher,
    at: i64,
) -> Result<CertificateSessionProvenance, ApplicationError> {
    let origin = provenance(&value.context, hasher, at)?;
    let captured = &value.statement;
    let expected = statement(
        &value.context,
        *captured.nonce(),
        captured.issued_at_unix_seconds(),
        captured.expires_at_unix_seconds(),
    )?;
    if *captured != expected
        || !captured.is_live_at(at)
        || captured.expires_at_unix_seconds() > origin.valid_until_unix_seconds
    {
        return Err(ApplicationError::InvalidCredentials);
    }
    Ok(origin)
}

pub(crate) fn evidence(
    value: &StoredCertificateLogin,
    signature: &Signature,
    check: &CredentialCheck,
    hasher: &dyn DocumentHasher,
    at: i64,
) -> Result<CertificateSessionProvenance, ApplicationError> {
    let origin = capture(value, hasher, at)?;
    if check.certificate != value.context.certificate
        || check.trust != value.context.trust.inspection
        || check.statement_digest != hasher.hash_bytes(&value.statement.canonical_bytes())
        || check.signature != *signature
        || signature.as_bytes().len() != 384
        || check.checked_at != at
        || check.valid_from != origin.valid_from_unix_seconds
        || check.valid_until != origin.valid_until_unix_seconds
    {
        return Err(ApplicationError::InvalidCredentials);
    }
    Ok(origin)
}
