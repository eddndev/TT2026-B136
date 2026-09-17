use super::{
    sources,
    time::{utc, DeclaredTime},
};
use crate::error::ApiError;
use application::hearing_results::*;
use domain::{cases::CaseId, hearings::HearingId};
use serde_json::{json, Value};
pub(super) fn values(v: &HearingResultValues) -> Result<Value, ApiError> {
    Ok(
        json!({"occurrence":v.occurrence().as_str(),"extent":v.extent().as_str(),"event_time":DeclaredTime::try_from(v.event_time())?,"summary":v.summary().as_str(),
        "attendees":v.attendees().iter().map(|a|json!({"participant_id":a.participant_id().to_string(),"revision":a.revision().get(),"capacity":a.capacity().as_str(),"observation":a.observation().map(|s|s.as_str())})).collect::<Vec<_>>(),
        "agreements":v.agreements().iter().map(|a|json!({"id":a.id().to_string(),"text":a.text().as_str()})).collect::<Vec<_>>(),
        "provenance":{"kind":v.provenance().kind().as_str(),"reference":v.provenance().reference().map(|s|s.as_str()),"support":v.provenance().support().map(|s|json!({"document_id":s.reference().id.to_string(),"version":s.reference().version.get(),"digest":s.digest().to_hex()}))}}),
    )
}
pub(super) fn command(c: &HearingResultCommand) -> Result<Value, ApiError> {
    let mut change =
        json!({"action":c.action().as_str(),"expected_revision":c.expected_revision()});
    match &c.change {
        HearingResultChange::Record {
            anchor_revision,
            continuation,
            values: v,
        } => {
            change["anchor_revision"] = json!(anchor_revision.get());
            change["continuation"] = json!(continuation
                .map(|r| json!({"result_id":r.id().to_string(),"revision":r.revision().get()})));
            change["values"] = values(v)?;
        }
        HearingResultChange::Correct { values: v, .. } => change["values"] = values(v)?,
        HearingResultChange::Withdraw { .. } => {}
    }
    if let Some(reason) = c.reason() {
        change["reason"] = json!(reason.as_str());
    }
    Ok(
        json!({"operation_id":c.operation_id.to_string(),"hearing_id":c.hearing_id.to_string(),"result_id":c.result_id.to_string(),"change":change}),
    )
}
pub(super) fn receipt(r: &HearingResultReceipt) -> Value {
    json!({"operation_id":r.operation_id.to_string(),"action":r.action.as_str(),"expected_revision":r.expected_revision,"submission_digest":r.submission_digest.to_hex()})
}
pub(super) fn detail(
    row: HearingResultDetail,
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
    revision: Option<HearingResultRevision>,
) -> Result<Value, ApiError> {
    let s = &row.snapshot;
    if s.case_id != case
        || s.hearing_id != hearing
        || s.id != id
        || revision.is_some_and(|r| r != s.revision)
        || s.anchor != row.anchor.reference
        || s.continuation != row.continuation.map(|c| c.reference)
    {
        return Err(ApiError::internal());
    }
    sources::validate(
        case,
        hearing,
        id,
        &s.values,
        &row.anchor,
        row.continuation.as_ref(),
        &row.attendees,
        row.support.as_ref(),
    )?;
    Ok(
        json!({"case_id":case,"hearing_id":hearing.to_string(),"id":id.to_string(),"revision":s.revision.get(),"values":values(&s.values)?,"values_digest":s.values_digest.to_hex(),"status":s.status.as_str(),"reason":s.reason.as_ref().map(|r|r.as_str()),
        "receipt":receipt(&s.receipt),"anchor":sources::anchor(&row.anchor)?,"continuation":row.continuation.as_ref().map(sources::continuation),
        "recorded_administration_revision":s.recorded_administration_revision.get(),"recorded_administration_digest":s.recorded_administration_digest.to_hex(),"recorded_at":utc(s.recorded_at)?,"recorded_by":{"id":s.recorded_by.id,"email":s.recorded_by.email},
        "attendees":sources::attendees(&row.attendees,&s.values),"support":row.support.as_ref().map(sources::support)}),
    )
}
pub(super) fn overview(v: HearingResultOverview) -> Result<Value, ApiError> {
    Ok(
        json!({"case_id":v.case_id,"hearing_id":v.hearing_id.to_string(),"id":v.id.to_string(),"revision":v.revision.get(),"status":v.status.as_str(),"occurrence":v.occurrence.as_str(),"extent":v.extent.as_str(),"event_time":DeclaredTime::try_from(v.event_time)?,"attendee_count":v.attendee_count,"agreement_count":v.agreement_count,"anchor_revision":v.anchor_revision.get()}),
    )
}
pub(super) fn history_entry(
    e: HearingResultHistoryEntry,
    case: CaseId,
    hearing: HearingId,
    id: HearingResultId,
) -> Result<Value, ApiError> {
    if e.case_id != case || e.hearing_id != hearing || e.id != id {
        return Err(ApiError::internal());
    }
    Ok(
        json!({"case_id":case,"hearing_id":hearing.to_string(),"id":id.to_string(),"revision":e.revision.get(),"status":e.status.as_str(),"reason":e.reason.as_ref().map(|r|r.as_str()),"values_digest":e.values_digest.to_hex(),"receipt":receipt(&e.receipt),
        "anchor":sources::anchor_reference(&e.anchor),"continuation":e.continuation.as_ref().map(sources::continuation_reference),
        "recorded_administration_revision":e.recorded_administration_revision.get(),"recorded_administration_digest":e.recorded_administration_digest.to_hex(),"recorded_at":utc(e.recorded_at)?,"recorded_by":{"id":e.recorded_by.id,"email":e.recorded_by.email}}),
    )
}
