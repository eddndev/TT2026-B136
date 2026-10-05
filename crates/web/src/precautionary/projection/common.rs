use crate::{error::ApiError, typed_participants::projection as people};
use application::{
    case_stages::StageSupportSnapshot,
    cases::CaseAdministrationSnapshot,
    identity::Principal,
    precautionary_hearings::{PrecautionaryContext, PrecautionaryContextExpectation},
    precautionary_measures::{MeasureCaptureAction, MeasureGroupRef, MeasureOriginIds},
    procedural_facts::FactParticipantProjection,
};
use domain::{
    crypto::DocumentHasher, hearings::HearingSupportRef,
    precautionary_hearings::PrecautionaryMeasureRef, typed_participants::SubjectRevisionRef,
};
use serde_json::{json, Value};

pub(super) fn utc(value: time::OffsetDateTime) -> Result<String, ApiError> {
    people::time(value)
}
pub(super) fn actor(value: &Principal) -> Value {
    json!({"id":value.id.to_string(),"email":value.email,"role":value.role.as_str()})
}
pub(super) fn expectation(value: &PrecautionaryContextExpectation) -> Value {
    json!({"administration_revision":value.administration_revision.get(),
        "stage_revision":value.stage_revision.get(),"context_digest":value.context_digest.to_hex()})
}
pub(crate) fn context(
    value: &PrecautionaryContext,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    let m = value.material();
    let stage = crate::case_stages::response::entry(m.stage.clone())?;
    let digest = value.digest(hasher).to_hex();
    Ok(
        json!({"case_id":m.case_id.to_string(),"administration":administration(&m.administration)?,
        "stage":stage,"stage_administration":administration(&m.stage_administration)?,
        "context_digest":digest,"expectation":{"administration_revision":m.administration.revision.get(),
            "stage_revision":m.stage.stage_revision().get(),"context_digest":digest}}),
    )
}
fn administration(value: &CaseAdministrationSnapshot) -> Result<Value, ApiError> {
    let value = crate::case_administration::response::Administration::try_from(value.clone())?;
    serde_json::to_value(value).map_err(|_| ApiError::internal())
}
pub(super) fn support(value: &StageSupportSnapshot) -> Value {
    json!({"document_id":value.reference.id.to_string(),"version":value.reference.version.get(),
        "digest":value.digest.to_hex(),"name":value.name,"format":value.format.as_str(),
        "policy":value.policy.as_str()})
}
pub(super) fn support_ref(value: HearingSupportRef) -> Value {
    json!({"document_id":value.reference().id.to_string(),"version":value.reference().version.get(),
        "digest":value.digest().to_hex()})
}
pub(super) fn reference(value: PrecautionaryMeasureRef) -> Value {
    json!({"id":value.id().to_string(),"revision":value.revision().get(),
        "capture_digest":value.digest().to_hex()})
}
pub(super) fn subject_ref(value: SubjectRevisionRef) -> Value {
    json!({"id":value.id.to_string(),"revision":value.revision.get(),
        "values_digest":value.values_digest.to_hex()})
}
pub(super) fn participant(value: &FactParticipantProjection) -> Value {
    let s = &value.snapshot;
    json!({"snapshot":{"case_id":s.case_id.to_string(),
        "reference":{"participant_id":s.reference.id.to_string(),"revision":s.reference.revision.get()},
        "values_digest":s.values_digest.to_hex(),"status":s.status.as_str(),
        "subject":s.subject.map(subject_ref)},"overview":people::overview(value.overview.clone())})
}
pub(super) fn group_ref(value: &MeasureGroupRef) -> Value {
    json!({"operation_id":value.operation_id.to_string(),"decision_id":value.decision_id.to_string(),
        "group_digest":value.group_digest.to_hex()})
}
pub(super) fn origin_ids(value: MeasureOriginIds) -> Value {
    json!({"operation_id":value.operation_id.to_string(),"decision_id":value.decision_id.to_string()})
}
pub(super) fn action(value: MeasureCaptureAction) -> &'static str {
    match value {
        MeasureCaptureAction::Impose => "impose",
        MeasureCaptureAction::Confirm => "confirm",
        MeasureCaptureAction::Modify => "modify",
        MeasureCaptureAction::Revoke => "revoke",
        MeasureCaptureAction::Cease => "cease",
        MeasureCaptureAction::SubstituteOut => "substitute_out",
        MeasureCaptureAction::SubstituteIn => "substitute_in",
    }
}
