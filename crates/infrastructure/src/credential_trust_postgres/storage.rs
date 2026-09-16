use application::credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot};
use application::ApplicationError;
use domain::crypto::{CredentialTrustInspection, Sha256Digest};
use postgres::{GenericClient, Row};
use time::OffsetDateTime;
use uuid::Uuid;

const SELECT: &str = "SELECT a.deployment_id,a.root_der,a.root_fingerprint,
    r.revision,r.crl_der,r.crl_digest,r.crl_number::text AS crl_number,
    r.crl_this_update,r.crl_next_update,r.valid_from,r.valid_until,
    r.published_at_seconds,r.published_at_nanoseconds,r.published_by
    FROM participant_credential_trust_revisions r
    JOIN participant_credential_authority a ON a.deployment_id=r.deployment_id";

pub(crate) fn current<C: GenericClient>(
    client: &mut C,
) -> Result<Option<CredentialTrustSnapshot>, ApplicationError> {
    client
        .query_opt(&format!("{SELECT} ORDER BY r.revision DESC LIMIT 1"), &[])
        .map_err(port)?
        .map(decode)
        .transpose()
}

pub(crate) fn exact<C: GenericClient>(
    client: &mut C,
    deployment: Uuid,
    revision: CredentialTrustRevision,
) -> Result<Option<CredentialTrustSnapshot>, ApplicationError> {
    client
        .query_opt(
            &format!("{SELECT} WHERE r.deployment_id=$1 AND r.revision=$2"),
            &[&deployment, &i64::from(revision.get())],
        )
        .map_err(port)?
        .map(decode)
        .transpose()
}

fn decode(row: Row) -> Result<CredentialTrustSnapshot, ApplicationError> {
    let revision: i64 = row.get("revision");
    let number: String = row.get("crl_number");
    let root_der: Vec<u8> = row.get("root_der");
    let crl_der: Vec<u8> = row.get("crl_der");
    let root_fingerprint = digest(row.get("root_fingerprint"))?;
    let crl_digest = digest(row.get("crl_digest"))?;
    let seconds: i64 = row.get("published_at_seconds");
    let nanos: i32 = row.get("published_at_nanoseconds");
    let published_at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| inconsistent())?
        .replace_nanosecond(nanos.try_into().map_err(|_| inconsistent())?)
        .map_err(|_| inconsistent())?;
    let value = CredentialTrustInspection {
        root_der,
        crl_der,
        root_fingerprint,
        crl_digest,
        crl_number: number.parse().map_err(|_| inconsistent())?,
        crl_this_update: row.get("crl_this_update"),
        crl_next_update: row.get("crl_next_update"),
        valid_from: row.get("valid_from"),
        valid_until: row.get("valid_until"),
    };
    use domain::crypto::DocumentHasher;
    if value.root_der.is_empty()
        || value.root_der.len() > 16384
        || value.crl_der.is_empty()
        || value.crl_der.len() > 1048576
        || crate::RingSha256Hasher.hash_bytes(&value.root_der) != root_fingerprint
        || crate::RingSha256Hasher.hash_bytes(&value.crl_der) != crl_digest
        || value.crl_this_update < 0
        || value.crl_next_update <= value.crl_this_update
        || value.valid_from < value.crl_this_update
        || value.valid_until > value.crl_next_update
        || value.valid_from > value.valid_until
        || seconds < value.valid_from
        || seconds > value.valid_until
    {
        return Err(inconsistent());
    }
    let published_by: String = row.get("published_by");
    if published_by.is_empty() || published_by.len() > 1024 {
        return Err(inconsistent());
    }
    Ok(CredentialTrustSnapshot {
        deployment_id: row.get("deployment_id"),
        revision: CredentialTrustRevision::new(revision.try_into().map_err(|_| inconsistent())?)?,
        inspection: value,
        published_at,
        published_by,
    })
}

pub(super) fn require_publisher<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    let permitted: bool = client
        .query_one(
            "SELECT
        has_table_privilege(current_user,'participant_credential_authority','SELECT')
        AND has_table_privilege(current_user,'participant_credential_authority','INSERT')
        AND has_table_privilege(current_user,'participant_credential_trust_revisions','SELECT')
        AND has_table_privilege(current_user,'participant_credential_trust_revisions','INSERT')
        AND has_table_privilege(current_user,'audit_events','SELECT')
        AND has_table_privilege(current_user,'audit_events','INSERT')",
            &[],
        )
        .map_err(port)?
        .get(0);
    if !permitted {
        return Err(ApplicationError::InvalidConfiguration(
            "credential trust publication requires administrative insert privileges".into(),
        ));
    }
    Ok(())
}

fn digest(bytes: Vec<u8>) -> Result<Sha256Digest, ApplicationError> {
    Sha256Digest::from_bytes(&bytes).map_err(|_| inconsistent())
}
pub(super) fn inconsistent() -> ApplicationError {
    ApplicationError::InvalidConfiguration(
        "credential trust inventory is inconsistent; restore a consistent database".into(),
    )
}
pub(super) fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("credential trust database: {error}"))
}
