use super::{administrative, decision};
use crate::error::ApiError;
use application::precautionary_measures::MeasureDecisionRecordHistoryEvidence;
use domain::crypto::DocumentHasher;
use serde_json::{json, Value};

pub(super) fn check(value: &MeasureDecisionRecordHistoryEvidence) -> Result<(), ApiError> {
    application::measure_corrections::validate_measure_record_history_shape(value)
        .map_err(|_| ApiError::internal())
}

pub(super) fn project(
    value: &MeasureDecisionRecordHistoryEvidence,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    check(value)?;
    let groups = value
        .records
        .judicial
        .groups
        .iter()
        .map(|v| {
            Ok(json!({
                "origin":decision::origin(&v.origin),"capture":decision::group(&v.capture,h)?
            }))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    let administrations = value.records.administrative.iter().map(|v|Ok(json!({
        "origin":administrative::origin(&v.origin),"capture":administrative::capture(&v.capture,h)?
    }))).collect::<Result<Vec<_>, ApiError>>()?;
    let decisions = value
        .decisions
        .iter()
        .map(|v| {
            Ok(json!({
                "origin":decision::origin(&v.origin),"capture":decision::group_v2(&v.capture,h)?
            }))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    Ok(
        json!({"records":{"judicial":{"groups":groups},"administrative":administrations},"decisions":decisions}),
    )
}
