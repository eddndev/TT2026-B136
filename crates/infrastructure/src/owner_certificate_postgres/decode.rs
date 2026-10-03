use application::{
    credential_trust::CredentialTrustRevision, identity::owner_certificates::OwnerBindingReceipt,
    ApplicationError,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{CertificateSummary, CredentialCertificate, CredentialCheck, Sha256Digest, Signature},
    identity::UserId,
    owner_certificates::{BindingMaterial, BindingRecord, BindingStatement},
};
use postgres::{types::FromSql, GenericClient, Row};

use super::{account, inconsistent, validation};

pub(super) fn value<'a, T: FromSql<'a>>(row: &'a Row, name: &str) -> Result<T, ApplicationError> {
    row.try_get(name).map_err(|_| inconsistent())
}

pub(super) fn timestamp(seconds: i64, nanos: i32) -> Result<OffsetDateTime, ApplicationError> {
    let value = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| inconsistent())?
        .replace_nanosecond(nanos.try_into().map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())?;
    validation::time(value)?;
    Ok(value)
}

fn digest(row: &Row, name: &str) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(value::<&[u8]>(row, name)?).map_err(|_| inconsistent())
}

pub(super) fn registration<C: GenericClient>(
    client: &mut C,
    row: &Row,
) -> Result<OwnerBindingReceipt, ApplicationError> {
    let owner = UserId::from_uuid(value(row, "owner_id")?);
    let account = account::captured(
        owner,
        value(row, "account_revision")?,
        value(row, "auth_generation")?,
    )?;
    let revision: i64 = value(row, "trust_revision")?;
    let revision = CredentialTrustRevision::new(revision.try_into().map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())?;
    let deployment = value(row, "deployment_id")?;
    let trust = crate::credential_trust_postgres::exact(client, deployment, revision)
        .map_err(|_| inconsistent())?
        .ok_or_else(inconsistent)?;
    let material = BindingMaterial::new(
        deployment,
        value(row, "binding_id")?,
        digest(row, "root_fingerprint")?,
        digest(row, "leaf_fingerprint")?,
        revision.get(),
    )
    .map_err(|_| inconsistent())?;
    let statement = BindingStatement::new(account, owner, material).map_err(|_| inconsistent())?;
    if value::<&[u8]>(row, "statement")? != statement.canonical_bytes() {
        return Err(inconsistent());
    }
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
    let receipt = OwnerBindingReceipt {
        owner,
        record: BindingRecord::registered(statement),
        check,
        trust,
        registered_at: timestamp(
            value(row, "registered_at_seconds")?,
            value(row, "registered_at_nanoseconds")?,
        )?,
        withdrawn_at: None,
    };
    validation::receipt(&receipt)?;
    Ok(receipt)
}

pub(super) fn withdrawal(
    receipt: &mut OwnerBindingReceipt,
    row: &Row,
) -> Result<(), ApplicationError> {
    if value::<uuid::Uuid>(row, "binding_id")? != receipt.record.registration().material().binding()
    {
        return Err(inconsistent());
    }
    let account = account::captured(
        receipt.owner,
        value(row, "account_revision")?,
        value(row, "auth_generation")?,
    )?;
    receipt.record = receipt
        .record
        .withdraw(account, 1)
        .map_err(|_| inconsistent())?;
    let statement = receipt
        .record
        .withdrawal()
        .ok_or_else(inconsistent)?
        .canonical_bytes();
    if value::<&[u8]>(row, "statement")? != statement
        || digest(row, "statement_digest")? != validation::digest(&statement)
    {
        return Err(inconsistent());
    }
    receipt.withdrawn_at = Some(timestamp(
        value(row, "withdrawn_at_seconds")?,
        value(row, "withdrawn_at_nanoseconds")?,
    )?);
    validation::receipt(receipt)
}
