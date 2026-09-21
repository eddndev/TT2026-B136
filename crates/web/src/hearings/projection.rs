use super::time::{format_time, utc};
use crate::error::ApiError;
use application::{
    case_stages::{CaseStageEntry, StageSupportSnapshot},
    hearings::*,
};
use domain::cases::CaseId;
use serde_json::{json, Value};

pub(super) fn values(v: &HearingValues) -> Result<Value, ApiError> {
    Ok(
        json!({"kind":v.kind().as_str(),"scheduled_at":format_time(v.scheduled_at())?,"modality":v.modality().as_str(),"venue":v.venue().as_str(),"note":v.note().map(|n|n.as_str()),
        "participants":v.participants().iter().map(|p|json!({"participant_id":p.id().to_string(),"revision":p.revision().get()})).collect::<Vec<_>>(),
        "conviction_basis":v.conviction_basis().map(|b|json!({"statement":b.statement().as_str(),"support":{"document_id":b.support().reference().id.to_string(),"version":b.support().reference().version.get(),"digest":b.support().digest().to_hex()}}))}),
    )
}
pub(super) fn command(c: &HearingCommand) -> Result<Value, ApiError> {
    let mut change =
        json!({"action":c.action().as_str(),"expected_revision":c.expected_revision()});
    if let Some(ctx) = c.expected_context() {
        change["expected_case_revision"] = json!(ctx.case_revision.get());
        change["expected_stage_revision"] = json!(ctx.stage_revision.get());
    }
    match &c.change {
        HearingChange::Schedule { values: v, .. } | HearingChange::Replace { values: v, .. } => {
            change["values"] = values(v)?
        }
        HearingChange::Cancel { .. } => {}
    }
    if let Some(reason) = c.reason() {
        change["reason"] = json!(reason.as_str());
    }
    Ok(
        json!({"operation_id":c.operation_id.to_string(),"hearing_id":c.hearing_id.to_string(),"change":change}),
    )
}
pub(super) fn context(row: HearingCaseContext, case: CaseId) -> Result<Value, ApiError> {
    if row.case_id != case {
        return Err(ApiError::internal());
    }
    let admin = row.administration.values();
    let snapshot = row.administration.snapshot();
    let stage_digest = match row.stage.entry() {
        Some(CaseStageEntry::Changed(v)) => Some(v.values_digest.to_hex()),
        _ => None,
    };
    Ok(
        json!({"case_id":case,"case_revision":row.administration.revision().map_or(0,|r|r.get()),"case_values_digest":snapshot.map(|s|s.values_digest.to_hex()),
        "title":admin.metadata().title(),"reference":admin.metadata().reference(),"administrative_status":admin.status().as_str(),"profile_complete":admin.profile().is_some(),
        "stage_revision":row.stage.revision().map(|r|r.get()),"stage":row.stage.stage().map(|s|s.as_str()),"stage_values_digest":stage_digest}),
    )
}
pub(crate) fn detail(
    row: HearingDetail,
    case: CaseId,
    id: HearingId,
    revision: Option<HearingRevision>,
) -> Result<Value, ApiError> {
    let s = &row.snapshot;
    if s.case_id != case
        || s.id != id
        || revision.is_some_and(|r| r != s.revision)
        || row.participants.len() != s.values.participants().len()
    {
        return Err(ApiError::internal());
    }
    for (captured, expected) in row.participants.iter().zip(s.values.participants()) {
        if captured.overview.case_id != case
            || captured.overview.id != expected.id()
            || captured.overview.revision != expected.revision()
        {
            return Err(ApiError::internal());
        }
    }
    let ctx = s.scheduling_context;
    Ok(
        json!({"case_id":case,"id":id.to_string(),"revision":s.revision.get(),"values":values(&s.values)?,"values_digest":s.values_digest.to_hex(),"status":s.status.as_str(),"reason":s.reason.as_ref().map(|r|r.as_str()),
        "receipt":{"operation_id":s.receipt.operation_id.to_string(),"action":s.receipt.action.as_str(),"expected_revision":s.receipt.expected_revision,"expected_context":s.receipt.expected_context.map(|c|json!({"case_revision":c.case_revision.get(),"stage_revision":c.stage_revision.get()})),"submission_digest":s.receipt.submission_digest.to_hex()},
        "scheduling_context":{"administration_revision":ctx.administration_revision.get(),"administration_digest":ctx.administration_digest.to_hex(),"stage_revision":ctx.stage_revision.get(),"stage":ctx.stage.as_str(),"stage_digest":ctx.stage_digest.map(|d|d.to_hex())},
        "recorded_administration_revision":s.recorded_administration_revision.get(),"recorded_administration_digest":s.recorded_administration_digest.to_hex(),"recorded_at":utc(s.recorded_at)?,"recorded_by":{"id":s.recorded_by.id,"email":s.recorded_by.email},
        "participants":row.participants.iter().map(participant).collect::<Vec<_>>(),"support":row.support.as_ref().map(support)}),
    )
}
fn participant(value: &HearingParticipantSnapshot) -> Value {
    let p = &value.overview;
    json!({"id":p.id.to_string(),"revision":p.revision.get(),"profile":if p.kind.is_some(){"typed"}else{"manual"},"display_name":p.display_name,"procedural_role":p.procedural_role,"kind":p.kind.map(|k|k.as_str()),
        "subject":p.subject.map(|s|json!({"id":s.id.to_string(),"revision":s.revision.get(),"values_digest":s.values_digest.to_hex()})),"values_digest":value.values_digest.to_hex(),"directory_status":p.directory_status.as_str()})
}
fn support(s: &StageSupportSnapshot) -> Value {
    json!({"document_id":s.reference.id.to_string(),"version":s.reference.version.get(),"digest":s.digest.to_hex(),"name":s.name,"format":s.format.as_str(),"policy":s.policy.as_str()})
}
pub(crate) fn overview(v: HearingOverview) -> Result<Value, ApiError> {
    Ok(
        json!({"case_id":v.case_id,"case_title":v.case_title,"case_reference":v.case_reference,"case_status":v.case_status.as_str(),"id":v.id.to_string(),"revision":v.revision.get(),"kind":v.kind.as_str(),"scheduled_at":format_time(v.scheduled_at)?,"modality":v.modality.as_str(),"status":v.status.as_str(),"participant_count":v.participant_count}),
    )
}
