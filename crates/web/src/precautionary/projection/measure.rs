use super::{administrative, common::*, values};
use crate::{error::ApiError, typed_participants::projection as people};
use application::{measure_corrections::*, precautionary_measures::*};
use domain::crypto::DocumentHasher;
use serde_json::{json, Value};

pub(super) fn sources(value: &MeasureSources) -> Result<Value, ApiError> {
    Ok(json!({"subject":people::subject(&value.subject)?,
        "supervisor":value.supervisor.clone().map(people::detail).transpose()?}))
}
pub(super) fn projection(value: &MeasureSourceProjection) -> Value {
    let s = &value.subject;
    json!({"subject":{"case_id":s.case_id.to_string(),"id":s.id.to_string(),
        "revision":s.revision.get(),"kind":s.kind.as_str(),"display_name":s.display_name},
        "supervisor":value.supervisor.as_ref().map(participant)})
}
pub(super) fn root(value: &MeasureRecordRoot) -> Value {
    match value {
        MeasureRecordRoot::Judicial(ids) => json!({"kind":"judicial","origin":origin_ids(*ids)}),
        MeasureRecordRoot::Administrative {
            operation_id,
            measure_id,
        } => json!({"kind":"administrative",
            "operation_id":operation_id.to_string(),"measure_id":measure_id.to_string()}),
    }
}
pub(super) fn result(value: &ReviewedMeasureResult) -> Result<Value, ApiError> {
    Ok(
        json!({"id":value.id.to_string(),"revision":value.revision.get(),"origin":origin_ids(value.origin),
        "effect_key":value.effect_key.to_string(),"action":action(value.action),
        "previous":value.previous.map(reference),"values":values::measure(&value.values)?,
        "sources":sources(&value.sources)?,"projection":projection(&value.projection)}),
    )
}
pub(super) fn result_v2(value: &ReviewedMeasureResultV2) -> Result<Value, ApiError> {
    Ok(
        json!({"id":value.id.to_string(),"revision":value.revision.get(),"record_root":root(&value.record_root),
        "judicial_origin":origin_ids(value.judicial_origin),"effect_key":value.effect_key.to_string(),
        "action":action(value.action),"previous":value.previous.map(reference),
        "values":values::measure(&value.values)?,"sources":sources(&value.sources)?,
        "projection":projection(&value.projection)}),
    )
}
pub(super) fn capture(value: &MeasureCapture) -> Result<Value, ApiError> {
    Ok(
        json!({"family":"m1","case_id":value.case_id.to_string(),"result":result(&value.result)?,
        "operation_id":value.operation_id.to_string(),"decision_id":value.decision_id.to_string(),
        "decision_digest":value.decision_digest.to_hex(),"actor":actor(&value.actor),
        "recorded_at":utc(value.recorded_at)?,"capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(super) fn capture_v2(value: &MeasureCaptureV2) -> Result<Value, ApiError> {
    Ok(
        json!({"family":"m2","case_id":value.case_id.to_string(),"result":result_v2(&value.result)?,
        "operation_id":value.operation_id.to_string(),"decision_id":value.decision_id.to_string(),
        "decision_digest":value.decision_digest.to_hex(),"actor":actor(&value.actor),
        "recorded_at":utc(value.recorded_at)?,"capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(super) fn owned(value: &OwnedMeasureMaterial) -> Result<Value, ApiError> {
    Ok(json!({"family":"m1","owner":group_ref(&value.owner),"capture":capture(&value.capture)?}))
}
pub(super) fn owned_record(
    value: &OwnedMeasureRecord,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    match value {
        OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(v)) => owned(v),
        OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(v)) => Ok(json!({"family":"m2",
            "owner":group_ref(&v.owner),"capture":capture_v2(&v.capture)?})),
        OwnedMeasureRecord::Administrative { owner, capture } => Ok(json!({"family":"c1",
            "owner":{"operation_id":owner.operation_id.to_string(),"capture_digest":owner.capture_digest.to_hex()},
            "capture":administrative::record(capture,hasher)?})),
    }
}
