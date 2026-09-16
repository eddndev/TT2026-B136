use super::{
    profile::Profile,
    values::{Locator, SubjectRef, SubjectValues},
};
use crate::error::ApiError;
use application::typed_participants as m;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

pub(crate) fn time(value: OffsetDateTime) -> Result<String, ApiError> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())
}
pub(crate) fn actor(value: &m::ParticipantActorSnapshot) -> Value {
    json!({"id":value.id.to_string(),"email":value.email})
}
pub(crate) fn subject(value: &m::SubjectSnapshot) -> Result<Value, ApiError> {
    Ok(
        json!({"case_id":value.case_id.to_string(),"id":value.id.to_string(),
        "revision":value.revision.get(),"values":SubjectValues::from(&value.values),
        "values_digest":value.values_digest.to_hex(),"changed_at":time(value.changed_at)?,
        "changed_by":actor(&value.changed_by)}),
    )
}
pub(crate) fn credential_ref(value: &m::ParticipantCredentialRef) -> Value {
    json!({"participant_id":value.participant_id.to_string(),
        "participant_revision":value.participant_revision.get(),"statement_digest":value.statement_digest.to_hex()})
}
pub(crate) fn manual(row: m::ParticipantSnapshot) -> Result<Value, ApiError> {
    Ok(json!({
        "case_id":row.case_id.to_string(),"id":row.id.to_string(),"revision":row.revision.get(),
        "display_name":row.values.display_name(),"procedural_role":row.values.procedural_role(),
        "organization":row.values.organization(),"legal_status":row.values.legal_status(),
        "directory_status":row.values.directory_status(),"values_digest":row.values_digest.to_hex(),
        "changed_at":time(row.changed_at)?,"changed_by":actor(&row.changed_by),
        "canonical_format":"part1","profile":null,"subject":null,"credential_origin":null,
        "submission_digest":null,"submission_revision":null,
    }))
}
pub(crate) fn detail(row: m::ParticipantDetail) -> Result<Value, ApiError> {
    match row.revision {
        m::ParticipantRevisionSnapshot::Manual(value) => {
            if row.bound_subject.is_some() {
                return Err(ApiError::internal());
            }
            manual(*value)
        }
        m::ParticipantRevisionSnapshot::Typed(value) => {
            let bound = row.bound_subject.ok_or_else(ApiError::internal)?;
            let reference = value.values.subject();
            if bound.case_id != value.case_id
                || bound.id != reference.id
                || bound.revision != reference.revision
                || bound.values_digest != reference.values_digest
            {
                return Err(ApiError::internal());
            }
            let role = value.values.role();
            Ok(
                json!({"case_id":value.case_id.to_string(),"id":value.id.to_string(),"revision":value.revision.get(),
                    "display_name":bound.values.display_name(),"procedural_role":value.values.kind().as_str(),
                    "organization":role.organization(),"legal_status":role.legal_status(),
                    "directory_status":value.values.directory_status(),"values_digest":value.values_digest.to_hex(),
                    "changed_at":time(value.changed_at)?,"changed_by":actor(&value.changed_by),
                    "canonical_format":"part2","profile":Profile::from(role.profile()),
                    "role_support":Locator::from(role.role_support()),"subject":subject(&bound)?,
                    "credential_origin":value.credential_origin.as_ref().map(credential_ref),
                    "submission_digest":value.submission_digest.to_hex(),"submission_revision":value.submission_revision.get(),
                }),
            )
        }
    }
}
pub(crate) fn overview(row: m::ParticipantOverview) -> Value {
    json!({"case_id":row.case_id.to_string(),"id":row.id.to_string(),"revision":row.revision.get(),
        "display_name":row.display_name,"procedural_role":row.procedural_role,"organization":row.organization,
        "directory_status":row.directory_status,"kind":row.kind.map(|v| v.as_str()),
        "canonical_format":if row.kind.is_some() { "part2" } else { "part1" },
        "subject":row.subject.map(SubjectRef::from)})
}
