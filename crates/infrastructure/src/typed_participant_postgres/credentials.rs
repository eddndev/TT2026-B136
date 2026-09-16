use super::*;
use domain::crypto::{InternalDeclarationVerifier, Signature};
pub(super) fn read(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ParticipantId,
    version: ParticipantRevision,
    hasher: &dyn DocumentHasher,
) -> Result<ParticipantCredentialEvidence, ApplicationError> {
    let detail = crate::participant_postgres::storage::exact(tx, case, id, version, hasher)?;
    let ParticipantRevisionSnapshot::Typed(snapshot) = detail.revision else {
        return Err(ApplicationError::ParticipantCredentialNotFound);
    };
    let reference = snapshot
        .credential_origin
        .ok_or(ApplicationError::ParticipantCredentialNotFound)?;
    let row = tx
        .query_opt(
            "SELECT * FROM participant_credential_evidence WHERE participant_id=$1 AND revision=$2",
            &[
                &id.as_uuid(),
                &i64::from(reference.participant_revision.get()),
            ],
        )
        .map_err(port)?
        .ok_or_else(inconsistent)?;
    crate::typed_participant_schema::validate_credential_binding(
        tx,
        id.as_uuid(),
        i64::from(reference.participant_revision.get()),
    )
    .map_err(|_| inconsistent())?;
    let trust = crate::credential_trust_postgres::exact(
        tx,
        row.get("deployment_id"),
        CredentialTrustRevision::new(
            u32::try_from(row.get::<_, i64>("trust_revision")).map_err(|_| inconsistent())?,
        )?,
    )?
    .ok_or_else(inconsistent)?;
    let bytes: Vec<u8> = row.get("declaration");
    let statement = digest(row.get("statement_digest"))?;
    if bytes.len() != 218
        || hasher.hash_bytes(&bytes) != statement
        || statement != reference.statement_digest
    {
        return Err(inconsistent());
    }
    let signature =
        Signature::from_bytes(row.get::<_, Vec<u8>>("signature")).map_err(|_| inconsistent())?;
    let certificate: Vec<u8> = row.get("certificate_der");
    let checked_at: i64 = row.get("checked_at");
    let check = crate::certificates::InternalRsaDeclarationVerifier
        .verify(
            &statement,
            &certificate,
            &signature,
            &trust.inspection.root_der,
            &trust.inspection.crl_der,
            checked_at,
        )
        .map_err(|_| inconsistent())?;
    if check.trust != trust.inspection
        || check.valid_from != row.get::<_, i64>("valid_from")
        || check.valid_until != row.get::<_, i64>("valid_until")
        || check.certificate.fingerprint != digest(row.get("certificate_fingerprint"))?
    {
        return Err(inconsistent());
    }
    let at = OffsetDateTime::from_unix_timestamp(row.get("accepted_at_seconds"))
        .map_err(|_| inconsistent())?
        .replace_nanosecond(
            u32::try_from(row.get::<_, i32>("accepted_at_nanoseconds"))
                .map_err(|_| inconsistent())?,
        )
        .map_err(|_| inconsistent())?;
    let origin = storage::typed_detail(tx, case, id, reference.participant_revision, hasher)?;
    let ParticipantRevisionSnapshot::Typed(origin) = origin.revision else {
        return Err(inconsistent());
    };
    if at.unix_timestamp() < check.checked_at
        || at != origin.changed_at
        || at.unix_timestamp() < check.valid_from
        || at.unix_timestamp() > check.valid_until
    {
        return Err(inconsistent());
    }
    Ok(ParticipantCredentialEvidence {
        case_id: case,
        reference,
        subject: origin.values.subject(),
        declaration: bytes,
        check,
        trust,
        accepted_at: at,
        accepted_by: origin.changed_by,
    })
}
