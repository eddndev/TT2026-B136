use super::{common::*, measure, values};
use crate::error::ApiError;
use application::measure_corrections::*;
use domain::{cases::CaseId, crypto::DocumentHasher};
use serde_json::{json, Value};

fn command(value: &MeasureAdministrativeCommand, case: CaseId) -> Result<Value, ApiError> {
    let action = match &value.action {
        MeasureAdministrativeAction::Correct(v) => json!({"kind":"correct","values":{
            "conditions":v.conditions().as_str(),"validity":values::validity(v.validity())?,
            "supervision_text":v.supervision_text().as_str()}}),
        MeasureAdministrativeAction::MarkEnteredInError => json!({"kind":"entered_in_error"}),
        MeasureAdministrativeAction::MarkEnteredInErrorAndReplace {
            replacement_id,
            subject,
        } => json!({"kind":"replace_entered_in_error","replacement_id":replacement_id.to_string(),
                "subject":subject_ref(*subject)}),
    };
    Ok(
        json!({"case_id":case.to_string(),"operation_id":value.operation_id.to_string(),"target":reference(value.target),
        "context":expectation(&value.context),"reason":value.reason.as_str(),"action":action}),
    )
}
fn result(value: &MeasureAdministrativeResult) -> Result<Value, ApiError> {
    let validity = match value.validity {
        MeasureCaptureValidity::Valid => "valid",
        MeasureCaptureValidity::EnteredInError => "entered_in_error",
    };
    Ok(
        json!({"id":value.id.to_string(),"revision":value.revision.get(),"previous":reference(value.previous),
        "record_root":measure::root(&value.record_root),"judicial_origin":origin_ids(value.judicial_origin),
        "last_judicial":{"owner":group_ref(&value.last_judicial.owner),"reference":reference(value.last_judicial.reference)},
        "last_action":action(value.last_action),"validity":validity,"values":values::measure(&value.values)?,
        "sources":measure::sources(&value.sources)?,"projection":measure::projection(&value.projection)}),
    )
}
pub(super) fn review(
    value: &MeasureAdministrativeReview,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(
        json!({"case_id":value.case_id.to_string(),"actor":actor(&value.actor),"command":command(&value.command,value.case_id)?,
        "context":context(&value.context,h)?,"support":support(&value.support),"result":result(&value.result)?,
        "replacement":value.replacement.as_ref().map(result).transpose()?,
        "submission_digest":value.submission_digest.to_hex(),"review_digest":value.review_digest.to_hex()}),
    )
}
pub(super) fn record(
    value: &MeasureAdministrativeRecordCapture,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(
        json!({"family":"c1","case_id":value.case_id.to_string(),"operation_id":value.operation_id.to_string(),
        "result":result(&value.result)?,"actor":actor(&value.actor),"context":context(&value.context,h)?,
        "support":support(&value.support),"review_digest":value.review_digest.to_hex(),
        "recorded_at":utc(value.recorded_at)?,"capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(super) fn capture(
    value: &MeasureAdministrativeCapture,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    let records = value
        .records
        .iter()
        .map(|v| record(v, h))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        json!({"family":"a1","review":review(&value.review,h)?,"records":records,
        "replacement_link":value.replacement_link.as_ref().map(|v|json!({
            "entered_in_error":reference(v.entered_in_error),"replacement":reference(v.replacement)})),
        "recorded_at":utc(value.recorded_at)?,"capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(super) fn origin(value: &MeasureAdministrativeOrigin) -> Value {
    json!({"case_id":value.case_id.to_string(),"operation_id":value.operation_id.to_string(),
        "submission_digest":value.submission_digest.to_hex(),"review_digest":value.review_digest.to_hex(),
        "capture_digest":value.capture_digest.to_hex()})
}
