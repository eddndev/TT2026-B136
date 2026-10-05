use super::{decision, decision_bounds, history};
use crate::error::ApiError;
use application::precautionary_measures::*;
use domain::crypto::DocumentHasher;
use serde_json::{json, Value};

pub(crate) fn decision_review(
    value: &MeasureDecisionRecordReview,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(match value {
        MeasureDecisionRecordReview::V1(value) => {
            decision_bounds::review_v1(value)?;
            json!({"family":"g1","review":decision::review(value,hasher)?})
        }
        MeasureDecisionRecordReview::V2(value) => {
            decision_bounds::review_v2(value)?;
            json!({"family":"g2","review":decision::review_v2(value,hasher)?})
        }
    })
}

pub(crate) fn decision_operation(
    value: &MeasureDecisionRecordReceipt,
    hasher: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    decision_bounds::bound_decision(value, value.case_id(), None, None)?;
    Ok(match value {
        MeasureDecisionRecordReceipt::V1(value) => {
            let ancestors = value.measure_history.groups.iter().map(|v| {
                Ok(json!({"origin":decision::origin(&v.origin),"capture":decision::group(&v.capture,hasher)?}))
            }).collect::<Result<Vec<_>, ApiError>>()?;
            json!({"family":"g1","group":decision::group(&value.group,hasher)?,
                "origin":decision::origin(&value.origin),"measure_history":{"groups":ancestors}})
        }
        MeasureDecisionRecordReceipt::V2(value) => {
            json!({"family":"g2","group":decision::group_v2(&value.group,hasher)?,
                "origin":decision::origin(&value.origin),
                "record_history":history::project(&value.record_history,hasher)?})
        }
    })
}
