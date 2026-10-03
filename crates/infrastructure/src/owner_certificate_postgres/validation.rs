use application::{
    identity::owner_certificates::{
        OwnerBindingReceipt, OwnerCertificateError, PreparedOwnerRegistration,
    },
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialFailure, DocumentHasher, Sha256Digest, Signature},
    owner_certificates::OwnerAccount,
};

use super::inconsistent;

pub(super) fn digest(bytes: &[u8]) -> Sha256Digest {
    crate::RingSha256Hasher.hash_bytes(bytes)
}

pub(super) fn time(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(inconsistent());
    }
    Ok(())
}

pub(super) fn after(at: OffsetDateTime, lower: OffsetDateTime) -> Result<(), ApplicationError> {
    time(at)?;
    time(lower)?;
    if at < lower {
        return Err(inconsistent());
    }
    Ok(())
}

pub(super) fn registration_time(
    at: OffsetDateTime,
    lower: OffsetDateTime,
    check: &domain::crypto::CredentialCheck,
) -> Result<(), ApplicationError> {
    after(at, lower)?;
    let seconds = at.unix_timestamp();
    if seconds < check.checked_at {
        return Err(inconsistent());
    }
    if seconds < check.valid_from {
        return Err(OwnerCertificateError::Credential(CredentialFailure::NotYetValid).into());
    }
    if seconds > check.valid_until {
        return Err(OwnerCertificateError::Credential(CredentialFailure::Expired).into());
    }
    Ok(())
}

pub(super) fn receipt(value: &OwnerBindingReceipt) -> Result<(), ApplicationError> {
    let statement = value.record.registration();
    let material = statement.material();
    let check = &value.check;
    let certificate = &check.certificate;
    let summary = &certificate.summary;
    let trust = &value.trust;
    time(value.registered_at)?;
    time(trust.published_at)?;
    for seconds in [
        summary.not_before_unix,
        summary.not_after_unix,
        check.checked_at,
        check.valid_from,
        check.valid_until,
    ] {
        time(OffsetDateTime::from_unix_timestamp(seconds).map_err(|_| inconsistent())?)?;
    }
    if value.owner != statement.owner().id()
        || material.deployment() != trust.deployment_id
        || material.trust_revision() != trust.revision.get()
        || material.root() != trust.inspection.root_fingerprint
        || material.leaf() != certificate.fingerprint
        || certificate.der.is_empty()
        || certificate.der.len() > 16384
        || digest(&certificate.der) != certificate.fingerprint
        || summary.subject.is_empty()
        || summary.subject.len() > 65536
        || summary.issuer.is_empty()
        || summary.issuer.len() > 65536
        || summary.serial_hex.is_empty()
        || summary.serial_hex.len() > 40
        || !summary
            .serial_hex
            .bytes()
            .all(|v| v.is_ascii_digit() || (b'A'..=b'F').contains(&v))
        || summary.not_before_unix > summary.not_after_unix
        || check.statement_digest != digest(&statement.canonical_bytes())
        || check.signature.as_bytes().len() != 384
        || check.trust != trust.inspection
        || check.valid_from != summary.not_before_unix.max(trust.inspection.valid_from)
        || check.valid_until != summary.not_after_unix.min(trust.inspection.valid_until)
        || check.valid_from > check.valid_until
        || check.checked_at < check.valid_from
        || check.checked_at > check.valid_until
        || value.registered_at.unix_timestamp() < check.checked_at
        || value.registered_at.unix_timestamp() > check.valid_until
        || value.registered_at < trust.published_at
    {
        return Err(inconsistent());
    }
    match (value.record.withdrawal(), value.withdrawn_at) {
        (None, None) => {}
        (Some(_), Some(at)) => after(at, value.registered_at)?,
        _ => return Err(inconsistent()),
    }
    Ok(())
}

pub(super) fn intent(
    value: &OwnerBindingReceipt,
    prepared: &PreparedOwnerRegistration,
    signature: &Signature,
) -> Result<(), ApplicationError> {
    if value.owner != prepared.account().principal.id
        || value.record.registration() != prepared.statement()
        || value.check.certificate != *prepared.certificate()
        || value.check.signature != *signature
        || value.trust != *prepared.trust()
    {
        return Err(OwnerCertificateError::BindingConflict.into());
    }
    receipt(value)
}

pub(super) fn prepared(value: &PreparedOwnerRegistration) -> Result<(), ApplicationError> {
    let account = value.account();
    let captured = OwnerAccount::new(
        account.principal.id,
        account.principal.role,
        account.active,
        account.revision,
        account.auth_generation,
    )
    .map_err(|_| inconsistent())?;
    if value.statement().owner() != &captured {
        return Err(inconsistent());
    }
    Ok(())
}

pub(super) fn same_registration(left: &OwnerBindingReceipt, right: &OwnerBindingReceipt) -> bool {
    left.owner == right.owner
        && left.record.registration() == right.record.registration()
        && left.check == right.check
        && left.trust == right.trust
        && left.registered_at == right.registered_at
}

pub(super) fn resource(
    value: &OwnerBindingReceipt,
    withdrawal: bool,
) -> Result<String, ApplicationError> {
    let (revision, bytes) = if withdrawal {
        (
            2,
            value
                .record
                .withdrawal()
                .ok_or_else(inconsistent)?
                .canonical_bytes(),
        )
    } else {
        (1, value.record.registration().canonical_bytes())
    };
    Ok(format!(
        "owner-certificate:{}:owner:{}:revision:{revision}:statement:{}",
        value.record.registration().material().binding(),
        value.owner,
        digest(&bytes).to_hex()
    ))
}
