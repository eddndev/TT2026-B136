use super::common::*;
use crate::error::ApiError;
use domain::precautionary_measures::*;
use serde_json::{json, Value};

pub(super) fn measure(value: &MeasureValues) -> Result<Value, ApiError> {
    let supervision = match value.supervision() {
        MeasureSupervision::Known {
            participant,
            statement,
        } => json!({"kind":"known",
            "participant":{"participant_id":participant.id().to_string(),"revision":participant.revision().get()},
            "statement":statement.as_str()}),
        MeasureSupervision::Unknown { reason } => {
            json!({"kind":"unknown","reason":reason.as_str()})
        }
    };
    Ok(
        json!({"subject":subject_ref(value.subject()),"kind":value.kind().as_str(),
        "conditions":value.conditions().as_str(),"validity":validity(value.validity())?,
        "supervision":supervision}),
    )
}
pub(super) fn validity(value: &MeasureValidity) -> Result<Value, ApiError> {
    Ok(
        json!({"start":declared_time(value.start())?,"statement":value.statement().as_str(),
        "end":value.end().map(declared_time).transpose()?}),
    )
}
fn declared_time(value: &MeasureTime) -> Result<Value, ApiError> {
    let mut result = crate::procedural_facts::values::time::project(value.declared())?;
    if let Some(reason) = value.unknown_reason() {
        result["reason"] = json!(reason.as_str());
    }
    Ok(result)
}
pub(super) fn decision(value: &MeasureDecisionValues) -> Result<Value, ApiError> {
    Ok(
        json!({"authority":value.authority().as_str(),"declared_at":declared_time(value.declared_at())?,
        "justification":value.justification().as_str(),"support":support_ref(value.support()),
        "locator":value.locator().as_str()}),
    )
}
pub(super) fn outcome(value: &MeasureDecisionOutcome) -> Result<Value, ApiError> {
    if let Some(note) = value.no_measure_change() {
        return Ok(json!({"kind":"no_measure_change","statement":note.as_str()}));
    }
    let effects = value
        .changes()
        .ok_or_else(ApiError::internal)?
        .iter()
        .map(effect)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"kind":"changes","effects":effects}))
}
fn proposal(value: &MeasureProposal) -> Result<Value, ApiError> {
    Ok(json!({"id":value.id.to_string(),"values":measure(&value.values)?}))
}
fn effect(value: &MeasureEffect) -> Result<Value, ApiError> {
    Ok(match value {
        MeasureEffect::Impose(v) => json!({"action":"impose","proposal":proposal(v)?}),
        MeasureEffect::Confirm { previous } => {
            json!({"action":"confirm","previous":reference(*previous)})
        }
        MeasureEffect::Modify {
            previous,
            values: v,
        } => json!({"action":"modify",
            "previous":reference(*previous),"values":measure(v)?}),
        MeasureEffect::Revoke { previous } => {
            json!({"action":"revoke","previous":reference(*previous)})
        }
        MeasureEffect::Cease { previous } => {
            json!({"action":"cease","previous":reference(*previous)})
        }
        MeasureEffect::Substitute {
            predecessors,
            successors,
        } => json!({"action":"substitute",
            "predecessors":predecessors.iter().copied().map(reference).collect::<Vec<_>>(),
            "successors":successors.iter().map(proposal).collect::<Result<Vec<_>, _>>()?}),
    })
}
