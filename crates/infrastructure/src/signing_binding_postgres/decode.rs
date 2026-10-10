use super::{account, inconsistent};
use application::{
    credential_trust::CredentialTrustRevision,
    document_signing::{validate_binding_receipt, SigningBindingReceipt},
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{CertificateSummary, CredentialCertificate, CredentialCheck, Sha256Digest, Signature},
    document_signing::{SignerAccount, SigningBindingMaterial, SigningBindingStatement},
    identity::{Role, UserId},
};
use postgres::{types::FromSql, GenericClient, Row};

pub(super) fn value<'a, T: FromSql<'a>>(row: &'a Row, name: &str) -> Result<T, ApplicationError> {
    row.try_get(name).map_err(|_| inconsistent())
}
pub(super) fn timestamp(seconds: i64, nanos: i32) -> Result<OffsetDateTime, ApplicationError> {
    if !(0..=253402300799).contains(&seconds) {
        return Err(inconsistent());
    }
    OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| inconsistent())?
        .replace_nanosecond(nanos.try_into().map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())
}
fn digest(row: &Row, name: &str) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(value::<&[u8]>(row, name)?).map_err(|_| inconsistent())
}
pub(super) fn registration<C: GenericClient>(
    client: &mut C,
    row: &Row,
) -> Result<SigningBindingReceipt, ApplicationError> {
    let signer = UserId::from_uuid(value(row, "signer_id")?);
    let account = SignerAccount::new(
        signer,
        Role::Litigator,
        true,
        account::counter(value(row, "account_revision")?)?,
        account::counter(value(row, "auth_generation")?)?,
    )
    .map_err(|_| inconsistent())?;
    let revision: i64 = value(row, "trust_revision")?;
    let revision = CredentialTrustRevision::new(revision.try_into().map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())?;
    let deployment = value(row, "deployment_id")?;
    let trust = crate::credential_trust_postgres::exact(client, deployment, revision)?
        .ok_or_else(inconsistent)?;
    let material = SigningBindingMaterial::new(
        deployment,
        value(row, "binding_id")?,
        digest(row, "root_fingerprint")?,
        digest(row, "leaf_fingerprint")?,
        revision.get(),
    )
    .map_err(|_| inconsistent())?;
    let statement =
        SigningBindingStatement::new(account, signer, material).map_err(|_| inconsistent())?;
    statement
        .require_exact_bytes(value(row, "statement")?)
        .map_err(|_| inconsistent())?;
    let certificate = CredentialCertificate {
        der: value(row, "certificate_der")?,
        fingerprint: material.leaf(),
        summary: CertificateSummary {
            subject: value(row, "certificate_subject")?,
            issuer: value(row, "certificate_issuer")?,
            serial_hex: value(row, "certificate_serial")?,
            not_before_unix: value(row, "certificate_not_before")?,
            not_after_unix: value(row, "certificate_not_after")?,
        },
    };
    let check = CredentialCheck {
        certificate,
        trust: trust.inspection.clone(),
        statement_digest: digest(row, "statement_digest")?,
        signature: Signature::from_bytes(value(row, "signature")?).map_err(|_| inconsistent())?,
        checked_at: value(row, "checked_at")?,
        valid_from: value(row, "valid_from")?,
        valid_until: value(row, "valid_until")?,
    };
    let receipt = SigningBindingReceipt {
        statement,
        check,
        trust,
        registered_at: timestamp(
            value(row, "registered_at_seconds")?,
            value(row, "registered_at_nanoseconds")?,
        )?,
        withdrawn_at: None,
    };
    validate_binding_receipt(
        &crate::RingSha256Hasher,
        &receipt,
        signer,
        material.binding(),
    )?;
    Ok(receipt)
}
pub(super) fn withdrawal(
    receipt: &mut SigningBindingReceipt,
    row: &Row,
) -> Result<(), ApplicationError> {
    let original = receipt.statement.account();
    let revision = account::counter(value(row, "account_revision")?)?;
    let generation = account::counter(value(row, "auth_generation")?)?;
    SignerAccount::new(original.id(), Role::Litigator, true, revision, generation)
        .map_err(|_| inconsistent())?;
    if revision < original.revision()
        || generation < original.generation()
        || value::<uuid::Uuid>(row, "binding_id")? != receipt.statement.material().binding()
    {
        return Err(inconsistent());
    }
    receipt.withdrawn_at = Some(timestamp(
        value(row, "withdrawn_at_seconds")?,
        value(row, "withdrawn_at_nanoseconds")?,
    )?);
    validate_binding_receipt(
        &crate::RingSha256Hasher,
        receipt,
        original.id(),
        receipt.statement.material().binding(),
    )
}
