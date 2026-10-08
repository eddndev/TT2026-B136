use super::{common::*, history};
use crate::{error::ApiError, typed_participants::projection as people};
use application::precautionary_hearings::*;
use domain::{
    cases::CaseId, crypto::DocumentHasher, precautionary_hearings::PrecautionaryHearingValues,
};
use serde_json::{json, Value};

pub(crate) fn command(value: &PrecautionaryHearingCommand, case: CaseId) -> Value {
    let change = match &value.change {
        PrecautionaryHearingChange::Schedule { context, values: v } => {
            json!({"action":"schedule","context":expectation(context),"values":values(v)})
        }
        PrecautionaryHearingChange::Replace {
            expected_revision,
            expected_capture_digest,
            context,
            values: v,
            reason,
        } => json!({"action":"replace","expected_revision":expected_revision.get(),
            "expected_capture_digest":expected_capture_digest.to_hex(),"context":expectation(context),
            "values":values(v),"reason":reason.as_str()}),
        PrecautionaryHearingChange::Cancel {
            expected_revision,
            expected_capture_digest,
            reason,
        } => json!({"action":"cancel","expected_revision":expected_revision.get(),
                "expected_capture_digest":expected_capture_digest.to_hex(),"reason":reason.as_str()}),
    };
    json!({"case_id":case.to_string(),"operation_id":value.operation_id.to_string(),
        "hearing_id":value.hearing_id.to_string(),"change":change})
}
fn values(value: &PrecautionaryHearingValues) -> Value {
    let basis = value.scheduling_basis();
    let at = value.scheduled_at().value();
    let offset = at.offset().whole_seconds();
    let suffix = if offset == 0 {
        "Z".to_owned()
    } else {
        format!(
            "{}{:02}:{:02}",
            if offset < 0 { '-' } else { '+' },
            offset.abs() / 3600,
            offset.abs() % 3600 / 60
        )
    };
    let scheduled = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{suffix}",
        at.year(),
        at.month() as u8,
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    );
    json!({"purpose":value.purpose().as_str(),"scheduled_at":scheduled,
        "modality":value.modality().as_str(),"venue":value.venue().as_str(),
        "note":value.note().map(|v|v.as_str()),
        "participants":value.participants().iter().map(|p|json!({"participant_id":p.id().to_string(),
            "revision":p.revision().get()})).collect::<Vec<_>>(),
        "scheduling_basis":{"statement":basis.statement().as_str(),"support":support_ref(basis.support()),
            "locator":basis.locator().as_str()},
        "review_targets":value.review_targets().iter().copied().map(reference).collect::<Vec<_>>()})
}
pub(crate) fn review(
    value: &PrecautionaryHearingReview,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    let participants = value
        .sources
        .participants
        .iter()
        .cloned()
        .map(people::detail)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"case_id":value.case_id.to_string(),"actor":actor(&value.actor),
        "command":super::command(&value.command,value.case_id),"resolved_values":values(&value.resolved_values),
        "result_revision":value.result_revision.get(),"status":value.status.as_str(),
        "scheduling_context":context(&value.scheduling_context,hasher)?,
        "observed_context":context(&value.observed_context,hasher)?,
        "sources":{"participants":participants,"support":support(&value.sources.support)},
        "participants":value.participants.iter().map(participant).collect::<Vec<_>>(),
        "submission_digest":value.submission_digest.to_hex(),"review_digest":value.review_digest.to_hex()}),
    )
}
pub(super) fn capture(
    value: &PrecautionaryHearingCapture,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(
        json!({"review":review(&value.review,h)?,"recorded_at":utc(value.recorded_at)?,
        "capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(crate) fn operation(
    value: &PrecautionaryHearingRecordStoredOperation,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    history::check(&value.history.record_history)?;
    let captures = value
        .history
        .captures
        .iter()
        .map(|v| capture(v, hasher))
        .collect::<Result<Vec<_>, _>>()?;
    let o = &value.history.origin;
    Ok(json!({"capture":capture(&value.capture,hasher)?,"history":{
        "origin":{"case_id":o.case_id.to_string(),"hearing_id":o.hearing_id.to_string(),
            "operation_id":o.operation_id.to_string(),"revision":o.revision.get(),
            "submission_digest":o.submission_digest.to_hex(),"review_digest":o.review_digest.to_hex(),
            "capture_digest":o.capture_digest.to_hex()},"captures":captures,
        "record_history":history::project(&value.history.record_history,hasher)?}}))
}
