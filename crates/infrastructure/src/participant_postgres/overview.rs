use application::{participants::ParticipantOverview, ApplicationError};
use domain::{cases::CaseId, crypto::Sha256Digest, typed_participants::*};
use postgres::Row;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

pub(super) fn decode(row: &Row) -> Result<ParticipantOverview, ApplicationError> {
    if !row.try_get::<_, bool>("integrity_ok").map_err(invalid)?
        || !row
            .try_get::<_, bool>("subject_integrity_ok")
            .map_err(invalid)?
    {
        return Err(invalid("participant overview failed integrity validation"));
    }
    canonical_time(row.try_get("changed_at").map_err(invalid)?)?;
    let case_id = CaseId::from_uuid(row.try_get("case_id").map_err(invalid)?);
    let id = ParticipantId::from_uuid(row.try_get("participant_id").map_err(invalid)?);
    let revision = super::storage::revision(row.try_get("revision").map_err(invalid)?)?;
    let name: String = row.try_get("overview_name").map_err(invalid)?;
    let checked = ParticipantText::<200>::new(&name).map_err(invalid)?;
    if checked.as_str() != name {
        return Err(invalid("noncanonical participant name"));
    }
    let role: String = row.try_get("procedural_role").map_err(invalid)?;
    let role_checked = ParticipantText::<80>::new(&role).map_err(invalid)?;
    if role_checked.as_str() != role {
        return Err(invalid("noncanonical participant role"));
    }
    let organization: Option<String> = row.try_get("organization").map_err(invalid)?;
    if let Some(value) = &organization {
        if ParticipantText::<200>::new(value)
            .map_err(invalid)?
            .as_str()
            != value
        {
            return Err(invalid("noncanonical participant organization"));
        }
    }
    let directory_status: DirectoryStatus = row
        .try_get::<_, String>("directory_status")
        .map_err(invalid)?
        .parse()
        .map_err(invalid)?;
    let typed: bool = row.try_get("typed").map_err(invalid)?;
    if revision == ParticipantRevision::initial() && directory_status != DirectoryStatus::Active {
        return Err(invalid("initial participant revision is not active"));
    }
    let (kind, subject) = if typed {
        canonical_time(row.try_get("subject_changed_at").map_err(invalid)?)?;
        let bound_case = CaseId::from_uuid(row.try_get("subject_case_id").map_err(invalid)?);
        if bound_case != case_id {
            return Err(invalid("participant subject belongs to another case"));
        }
        let digest: Vec<u8> = row.try_get("subject_values_digest").map_err(invalid)?;
        let subject = SubjectRevisionRef {
            id: CaseSubjectId::from_uuid(row.try_get("subject_id").map_err(invalid)?),
            revision: SubjectRevision::new(
                u32::try_from(row.try_get::<_, i64>("subject_revision").map_err(invalid)?)
                    .map_err(invalid)?,
            )
            .map_err(invalid)?,
            values_digest: Sha256Digest::from_bytes(&digest).map_err(invalid)?,
        };
        (Some(kind(&role)?), Some(subject))
    } else {
        (None, None)
    };
    Ok(ParticipantOverview {
        case_id,
        id,
        revision,
        display_name: name,
        procedural_role: role,
        organization,
        directory_status,
        kind,
        subject,
    })
}
fn canonical_time(value: &str) -> Result<(), ApplicationError> {
    let at = OffsetDateTime::parse(value, &Rfc3339).map_err(invalid)?;
    if at
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(invalid)?
        != value
    {
        return Err(invalid("noncanonical participant timestamp"));
    }
    Ok(())
}
fn kind(value: &str) -> Result<ParticipantKind, ApplicationError> {
    use ParticipantKind as K;
    Ok(match value {
        "defendant" => K::Defendant,
        "victim" => K::Victim,
        "defense_counsel" => K::DefenseCounsel,
        "prosecutor" => K::Prosecutor,
        "victim_counsel" => K::VictimCounsel,
        "control_judge" => K::ControlJudge,
        "trial_court" => K::TrialCourt,
        "expert" => K::Expert,
        "police" => K::Police,
        "precautionary_supervisor" => K::PrecautionarySupervisor,
        "other" => K::Other,
        _ => return Err(invalid("unknown typed participant kind")),
    })
}
fn invalid(_: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::StoredParticipantInconsistent(
        "stored participant overview is inconsistent".into(),
    )
}
