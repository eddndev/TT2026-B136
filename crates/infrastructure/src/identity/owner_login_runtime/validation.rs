use application::{
    identity::certificate_login::{CertificateLoginContext, StoredCertificateLogin},
    members::{validate_user_summary, UserSummary},
};
use domain::{
    clock::OffsetDateTime,
    crypto::DocumentHasher,
    owner_certificate_login::{LoginAccount, LoginNonce, LoginStatement},
    owner_certificates::{BindingMaterial, OwnerAccount},
};

use crate::RingSha256Hasher;

const MAX_EXACT: i64 = 9_007_199_254_740_991;

pub(super) fn capture(value: &StoredCertificateLogin) -> Option<(i64, i64)> {
    context(&value.context)?;
    let actual = &value.statement;
    let issued = actual.issued_at_unix_seconds();
    let expires = actual.expires_at_unix_seconds();
    let rebuilt = statement(&value.context, *actual.nonce(), issued, expires)?;
    if actual != &rebuilt {
        return None;
    }
    let certificate = &value.context.certificate.summary;
    let trust = &value.context.trust;
    let from = certificate.not_before_unix.max(trust.inspection.valid_from);
    let until = certificate.not_after_unix.min(trust.inspection.valid_until);
    if issued < from || expires > until || trust.published_at.unix_timestamp() > issued {
        return None;
    }
    Some((milliseconds(issued)?, milliseconds(expires)?))
}

pub(super) fn statement(
    context: &CertificateLoginContext,
    nonce: LoginNonce,
    issued: i64,
    expires: i64,
) -> Option<LoginStatement> {
    let account = &context.account;
    let owner = LoginAccount::new(
        account.principal.id,
        account.principal.role,
        account.active,
        account.auth_generation,
    )
    .ok()?;
    let material = BindingMaterial::new(
        context.trust.deployment_id,
        context.binding_id,
        context.trust.inspection.root_fingerprint,
        context.certificate.fingerprint,
        context.trust.revision.get(),
    )
    .ok()?;
    LoginStatement::new(owner, material, nonce, issued, expires).ok()
}

fn context(value: &CertificateLoginContext) -> Option<()> {
    let account = &value.account;
    OwnerAccount::new(
        account.principal.id,
        account.principal.role,
        account.active,
        account.revision,
        account.auth_generation,
    )
    .ok()?;
    validate_user_summary(&UserSummary {
        id: account.principal.id,
        email: account.principal.email.clone(),
        role: account.principal.role,
        active: account.active,
        revision: account.revision,
    })
    .ok()?;
    let certificate = &value.certificate;
    let summary = &certificate.summary;
    let trust = &value.trust;
    let inspected = &trust.inspection;
    if value.binding_id.is_nil()
        || trust.deployment_id.is_nil()
        || certificate.der.is_empty()
        || certificate.der.len() > 16_384
        || inspected.root_der.is_empty()
        || inspected.root_der.len() > 16_384
        || inspected.crl_der.is_empty()
        || inspected.crl_der.len() > 1_048_576
        || summary.subject.is_empty()
        || summary.subject.len() > 65_536
        || summary.issuer.is_empty()
        || summary.issuer.len() > 65_536
        || summary.serial_hex.is_empty()
        || summary.serial_hex.len() > 40
        || !summary
            .serial_hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'A'..=b'F').contains(&byte))
        || summary.not_before_unix > summary.not_after_unix
        || trust.published_by.is_empty()
        || trust.published_by.len() > 1024
        || inspected.crl_this_update < 0
        || inspected.crl_this_update >= inspected.crl_next_update
        || inspected.valid_from < inspected.crl_this_update
        || inspected.valid_until > inspected.crl_next_update
        || inspected.valid_from >= inspected.valid_until
        || trust.published_at.unix_timestamp() < inspected.valid_from
        || trust.published_at.unix_timestamp() > inspected.valid_until
        || trust.published_at.offset() != time::UtcOffset::UTC
    {
        return None;
    }
    for seconds in [
        summary.not_before_unix,
        summary.not_after_unix,
        inspected.crl_this_update,
        inspected.crl_next_update,
        inspected.valid_from,
        inspected.valid_until,
        trust.published_at.unix_timestamp(),
    ] {
        let at = OffsetDateTime::from_unix_timestamp(seconds).ok()?;
        if !(1..=9999).contains(&at.year()) {
            return None;
        }
    }
    if RingSha256Hasher.hash_bytes(&certificate.der) != certificate.fingerprint
        || RingSha256Hasher.hash_bytes(&inspected.root_der) != inspected.root_fingerprint
        || RingSha256Hasher.hash_bytes(&inspected.crl_der) != inspected.crl_digest
    {
        return None;
    }
    Some(())
}

fn milliseconds(seconds: i64) -> Option<i64> {
    seconds
        .checked_mul(1000)
        .filter(|value| (0..=MAX_EXACT).contains(value))
}
