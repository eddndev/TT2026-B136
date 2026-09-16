use crate::error::ApiError;
use application::{case_stages::StageSupportSnapshot, hearing_results::*};
use domain::{cases::CaseId, hearings::HearingId};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
pub(super) fn anchor_reference(a: &HearingResultAnchor) -> Value {
    json!({"hearing_id":a.hearing_id.to_string(),"revision":a.revision.get(),"values_digest":a.values_digest.to_hex(),"submission_digest":a.submission_digest.to_hex()})
}
pub(super) fn continuation_reference(c: &HearingResultContinuation) -> Value {
    json!({"hearing_id":c.hearing_id.to_string(),"result_id":c.result_id.to_string(),"revision":c.revision.get(),"values_digest":c.values_digest.to_hex(),"submission_digest":c.submission_digest.to_hex()})
}
pub(super) fn anchor(a: &HearingResultAnchorSnapshot) -> Result<Value, ApiError> {
    let mut value = anchor_reference(&a.reference);
    let context = a.scheduling_context;
    value["status"] = json!(a.status.as_str());
    value["kind"] = json!(a.kind.as_str());
    value["scheduled_at"] = json!(a
        .scheduled_at
        .value()
        .format(&Rfc3339)
        .map_err(|_| ApiError::internal())?);
    value["scheduling_context"] = json!({"administration_revision":context.administration_revision.get(),"administration_digest":context.administration_digest.to_hex(),"stage_revision":context.stage_revision.get(),"stage":context.stage.as_str(),"stage_digest":context.stage_digest.map(|d|d.to_hex())});
    Ok(value)
}
pub(super) fn continuation(c: &HearingResultContinuationSnapshot) -> Value {
    let mut value = continuation_reference(&c.reference);
    value["status"] = json!(c.status.as_str());
    value
}
pub(super) fn support(s: &StageSupportSnapshot) -> Value {
    json!({"document_id":s.reference.id.to_string(),"version":s.reference.version.get(),"digest":s.digest.to_hex(),"name":s.name,"format":s.format.as_str(),"policy":s.policy.as_str()})
}
pub(super) fn attendees(
    items: &[HearingResultAttendeeSnapshot],
    values: &HearingResultValues,
) -> Vec<Value> {
    items.iter().zip(values.attendees()).map(|(a,declared)|{let p=&a.participant.overview;
        json!({"id":p.id.to_string(),"revision":p.revision.get(),"profile":if p.kind.is_some(){"typed"}else{"manual"},"display_name":p.display_name,"procedural_role":p.procedural_role,"kind":p.kind.map(|k|k.as_str()),"subject":p.subject.map(|s|json!({"id":s.id.to_string(),"revision":s.revision.get(),"values_digest":s.values_digest.to_hex()})),"values_digest":a.participant.values_digest.to_hex(),"subject_digest":a.subject_digest.map(|d|d.to_hex()),"directory_status":p.directory_status.as_str(),"capacity":declared.capacity().as_str(),"observation":declared.observation().map(|s|s.as_str())})
    }).collect()
}
#[allow(clippy::too_many_arguments)]
pub(super) fn validate(
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
    values: &HearingResultValues,
    anchor: &HearingResultAnchorSnapshot,
    continuation: Option<&HearingResultContinuationSnapshot>,
    attendees: &[HearingResultAttendeeSnapshot],
    support: Option<&StageSupportSnapshot>,
) -> Result<(), ApiError> {
    if anchor.reference.hearing_id != hearing
        || continuation.is_some_and(|c| c.reference.result_id == id)
        || attendees.len() != values.attendees().len()
    {
        return Err(ApiError::internal());
    }
    for (a, expected) in attendees.iter().zip(values.attendees()) {
        let p = &a.participant.overview;
        if p.case_id != case
            || p.id != expected.participant_id()
            || p.revision != expected.revision()
            || p.kind.is_some() != p.subject.is_some()
            || p.subject.map(|s| s.values_digest) != a.subject_digest
        {
            return Err(ApiError::internal());
        }
    }
    match (values.provenance().support(), support) {
        (None, None) => Ok(()),
        (Some(expected), Some(actual))
            if expected.reference() == actual.reference && expected.digest() == actual.digest =>
        {
            Ok(())
        }
        _ => Err(ApiError::internal()),
    }
}
