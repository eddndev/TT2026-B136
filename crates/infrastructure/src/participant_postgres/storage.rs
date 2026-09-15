use application::identity::Principal;
use application::participants::{
    participant_digest, DirectoryStatus, ParticipantActorSnapshot, ParticipantId,
    ParticipantRevision, ParticipantSnapshot, ParticipantValues,
};
use application::ApplicationError;
use domain::cases::CaseId;
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::identity::UserId;
use postgres::{Row, Transaction};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

pub(super) fn require_root(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ParticipantId,
) -> Result<(), ApplicationError> {
    tx.query_opt(
        "SELECT id FROM case_participants WHERE id=$1 AND case_id=$2",
        &[&id.as_uuid(), &case.as_uuid()],
    )
    .map_err(port)?
    .ok_or(ApplicationError::ParticipantNotFound)?;
    Ok(())
}

pub(super) fn current(
    tx: &mut Transaction<'_>,
    case: CaseId,
    id: ParticipantId,
    hasher: &dyn DocumentHasher,
) -> Result<ParticipantSnapshot, ApplicationError> {
    require_root(tx, case, id)?;
    let row = tx.query_opt("SELECT p.case_id,r.* FROM case_participant_revisions r JOIN case_participants p ON p.id=r.participant_id
        WHERE p.id=$1 AND p.case_id=$2 ORDER BY r.revision DESC LIMIT 1", &[&id.as_uuid(),&case.as_uuid()])
        .map_err(port)?.ok_or_else(|| inconsistent("participant root has no first revision"))?;
    decode(&row, hasher)
}

pub(super) fn snapshot(
    case_id: CaseId,
    id: ParticipantId,
    revision: ParticipantRevision,
    values: ParticipantValues,
    principal: &Principal,
    at: OffsetDateTime,
    hasher: &dyn DocumentHasher,
) -> ParticipantSnapshot {
    ParticipantSnapshot {
        case_id,
        id,
        revision,
        values_digest: participant_digest(hasher, &values),
        values,
        changed_at: at.to_offset(UtcOffset::UTC),
        changed_by: ParticipantActorSnapshot {
            id: principal.id,
            email: principal.email.clone(),
        },
    }
}

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    snapshot: &ParticipantSnapshot,
) -> Result<(), ApplicationError> {
    let v = &snapshot.values;
    let timestamp = canonical_time(snapshot.changed_at)?;
    tx.execute("INSERT INTO case_participant_revisions(participant_id,revision,display_name,procedural_role,organization,legal_status,directory_status,values_digest,changed_at,changed_by,changed_by_email)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
        &[&snapshot.id.as_uuid(),&i64::from(snapshot.revision.get()),&v.display_name(),&v.procedural_role(),&v.organization(),&v.legal_status(),
          &v.directory_status().as_str(),&&snapshot.values_digest.as_bytes()[..],&timestamp,&snapshot.changed_by.id.as_uuid(),&snapshot.changed_by.email])
        .map_err(port)?;
    Ok(())
}

pub(super) fn decode(
    row: &Row,
    hasher: &dyn DocumentHasher,
) -> Result<ParticipantSnapshot, ApplicationError> {
    let name: String = row.try_get("display_name").map_err(inconsistent)?;
    let role: String = row.try_get("procedural_role").map_err(inconsistent)?;
    let organization: Option<String> = row.try_get("organization").map_err(inconsistent)?;
    let legal: Option<String> = row.try_get("legal_status").map_err(inconsistent)?;
    let status: String = row.try_get("directory_status").map_err(inconsistent)?;
    let status: DirectoryStatus = status.parse().map_err(inconsistent)?;
    let values = ParticipantValues::new(
        &name,
        &role,
        organization.as_deref(),
        legal.as_deref(),
        status,
    )
    .map_err(inconsistent)?;
    if values.display_name() != name
        || values.procedural_role() != role
        || values.organization() != organization.as_deref()
        || values.legal_status() != legal.as_deref()
    {
        return Err(inconsistent("stored participant values are not canonical"));
    }
    let revision: i64 = row.try_get("revision").map_err(inconsistent)?;
    let revision = ParticipantRevision::new(u32::try_from(revision).map_err(inconsistent)?)
        .map_err(inconsistent)?;
    if revision == ParticipantRevision::initial() && status != DirectoryStatus::Active {
        return Err(inconsistent("first participant revision must be active"));
    }
    let digest: Vec<u8> = row.try_get("values_digest").map_err(inconsistent)?;
    let digest: [u8; 32] = digest
        .try_into()
        .map_err(|_| inconsistent("invalid participant digest length"))?;
    let values_digest = Sha256Digest::from_array(digest);
    if participant_digest(hasher, &values) != values_digest {
        return Err(inconsistent("participant values digest mismatch"));
    }
    let timestamp: String = row.try_get("changed_at").map_err(inconsistent)?;
    let changed_at = OffsetDateTime::parse(&timestamp, &Rfc3339).map_err(inconsistent)?;
    if timestamp != canonical_time(changed_at)? {
        return Err(inconsistent("participant timestamp is not canonical UTC"));
    }
    let email: String = row.try_get("changed_by_email").map_err(inconsistent)?;
    if email.is_empty() || email.trim() != email || email.chars().any(char::is_control) {
        return Err(inconsistent("participant actor email is not canonical"));
    }
    Ok(ParticipantSnapshot {
        case_id: CaseId::from_uuid(row.try_get("case_id").map_err(inconsistent)?),
        id: ParticipantId::from_uuid(row.try_get("participant_id").map_err(inconsistent)?),
        revision,
        values,
        values_digest,
        changed_at,
        changed_by: ParticipantActorSnapshot {
            id: UserId::from_uuid(row.try_get("changed_by").map_err(inconsistent)?),
            email,
        },
    })
}

pub(super) fn resource(snapshot: &ParticipantSnapshot) -> String {
    format!(
        "case:{}:participant:{}:revision:{}:sha256:{}",
        snapshot.case_id,
        snapshot.id,
        snapshot.revision.get(),
        snapshot.values_digest.to_hex()
    )
}
fn canonical_time(at: OffsetDateTime) -> Result<String, ApplicationError> {
    at.to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(inconsistent)
}
pub(super) fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("participant database: {error}"))
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::StoredParticipantInconsistent(error.to_string())
}
