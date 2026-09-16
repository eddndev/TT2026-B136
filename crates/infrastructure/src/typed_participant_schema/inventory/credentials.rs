use super::{digest, inconsistent, port};
use application::{credential_trust::CredentialTrustRevision, ApplicationError};
use domain::crypto::{InternalDeclarationVerifier, Signature};
use postgres::GenericClient;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

pub(super) fn validate<C: GenericClient>(client: &mut C) -> Result<(), ApplicationError> {
    relations(client, None, None)?;
    let mut id = Uuid::nil();
    let mut revision = 0_i64;
    loop {
        let rows=client.query("SELECT e.*,r.changed_at FROM participant_credential_evidence e
            JOIN case_participant_typed_revisions r ON r.participant_id=e.participant_id AND r.revision=e.revision
            WHERE (e.participant_id,e.revision)>($1,$2) ORDER BY e.participant_id,e.revision LIMIT 64",&[&id,&revision]).map_err(port)?;
        for row in &rows {
            let certificate: Vec<u8> =
                row.try_get("certificate_der").map_err(|_| inconsistent())?;
            let declaration: Vec<u8> = row.try_get("declaration").map_err(|_| inconsistent())?;
            let signature: Vec<u8> = row.try_get("signature").map_err(|_| inconsistent())?;
            if certificate.is_empty()
                || certificate.len() > 16384
                || declaration.len() != 218
                || signature.len() != 384
            {
                return Err(inconsistent());
            }
            let certificate_hash: Vec<u8> = row
                .try_get("certificate_fingerprint")
                .map_err(|_| inconsistent())?;
            digest(&certificate, &certificate_hash)?;
            let expected: Vec<u8> = row
                .try_get("statement_digest")
                .map_err(|_| inconsistent())?;
            let statement = digest(&declaration, &expected)?;
            let trust_revision: i64 = row.try_get("trust_revision").map_err(|_| inconsistent())?;
            let trust = crate::credential_trust_postgres::exact(
                client,
                row.get("deployment_id"),
                CredentialTrustRevision::new(
                    trust_revision.try_into().map_err(|_| inconsistent())?,
                )
                .map_err(|_| inconsistent())?,
            )?
            .ok_or_else(inconsistent)?;
            let check = crate::certificates::InternalRsaDeclarationVerifier::new()
                .verify(
                    &statement,
                    &certificate,
                    &Signature::from_bytes(signature).map_err(|_| inconsistent())?,
                    &trust.inspection.root_der,
                    &trust.inspection.crl_der,
                    row.get("checked_at"),
                )
                .map_err(|_| inconsistent())?;
            let accepted_seconds: i64 = row
                .try_get("accepted_at_seconds")
                .map_err(|_| inconsistent())?;
            let nanos: i32 = row
                .try_get("accepted_at_nanoseconds")
                .map_err(|_| inconsistent())?;
            let accepted = OffsetDateTime::from_unix_timestamp(accepted_seconds)
                .map_err(|_| inconsistent())?
                .replace_nanosecond(nanos.try_into().map_err(|_| inconsistent())?)
                .map_err(|_| inconsistent())?;
            if check.trust != trust.inspection
                || check.valid_from != row.get::<_, i64>("valid_from")
                || check.valid_until != row.get::<_, i64>("valid_until")
                || accepted_seconds < check.checked_at
                || accepted_seconds < check.valid_from
                || accepted_seconds > check.valid_until
                || accepted.format(&Rfc3339).map_err(|_| inconsistent())?
                    != row.get::<_, String>("changed_at")
            {
                return Err(inconsistent());
            }
            id = row.try_get("participant_id").map_err(|_| inconsistent())?;
            revision = row.try_get("revision").map_err(|_| inconsistent())?;
        }
        if rows.len() < 64 {
            return Ok(());
        }
    }
}

pub(crate) fn validate_binding<C: GenericClient>(
    client: &mut C,
    id: Uuid,
    revision: i64,
) -> Result<(), ApplicationError> {
    relations(client, Some(id), Some(revision))
}

fn relations<C: GenericClient>(
    client: &mut C,
    id: Option<Uuid>,
    revision: Option<i64>,
) -> Result<(), ApplicationError> {
    let broken:bool=client.query_one("SELECT EXISTS(SELECT 1 FROM participant_credential_evidence e
        LEFT JOIN case_participant_typed_revisions r ON r.participant_id=e.participant_id AND r.revision=e.revision
        LEFT JOIN case_participants p ON p.id=e.participant_id
        LEFT JOIN participant_identity_reviews v ON v.participant_id=e.participant_id AND v.revision=e.revision
        LEFT JOIN participant_credential_trust_revisions t ON t.deployment_id=e.deployment_id AND t.revision=e.trust_revision
        LEFT JOIN participant_credential_authority a ON a.deployment_id=e.deployment_id
        WHERE ($1::uuid IS NULL OR (e.participant_id=$1 AND e.revision=$2::bigint))
            AND (r.participant_id IS NULL OR v.participant_id IS NULL OR t.deployment_id IS NULL
            OR r.submission_revision<>e.revision OR r.credential_origin_revision IS DISTINCT FROM e.revision
            OR e.valid_from<t.valid_from OR e.valid_until>t.valid_until OR e.valid_from>e.valid_until
            OR e.checked_at>e.accepted_at_seconds
            OR substring(e.declaration FROM 1 FOR 8)<>decode('5043524544310000','hex')
            OR substring(e.declaration FROM 9 FOR 16)<>uuid_send(e.deployment_id)
            OR substring(e.declaration FROM 25 FOR 32)<>a.root_fingerprint
            OR substring(e.declaration FROM 57 FOR 16)<>uuid_send(p.case_id)
            OR substring(e.declaration FROM 73 FOR 16)<>uuid_send(r.subject_id)
            OR get_byte(e.declaration,88)<>get_byte(v.submission_canonical,21)
            OR substring(e.declaration FROM 90 FOR 40)<>substring(v.submission_canonical FROM 39 FOR 40)
            OR substring(e.declaration FROM 130 FOR 16)<>uuid_send(e.participant_id)
            OR typed_u32(e.declaration,145)<>e.revision-1 OR typed_u32(e.declaration,149)<>e.revision
            OR get_byte(e.declaration,153)<>array_position(ARRAY['defendant','victim','defense_counsel','prosecutor','victim_counsel',
                'control_judge','trial_court','expert','police','precautionary_supervisor','other'],r.role_kind)-1
            OR substring(e.declaration FROM 155 FOR 32)<>r.values_digest
            OR substring(e.declaration FROM 187 FOR 32)<>e.certificate_fingerprint
            OR octet_length(v.submission_canonical)<>615 OR get_byte(v.submission_canonical,166)<>1
            OR substring(v.submission_canonical FROM 168 FOR 32)<>e.statement_digest
            OR substring(v.submission_canonical FROM 200 FOR 32)<>e.certificate_fingerprint
            OR substring(v.submission_canonical FROM 232 FOR 384)<>e.signature))",&[&id,&revision]).map_err(port)?.get(0);
    if broken {
        return Err(inconsistent());
    }
    Ok(())
}
