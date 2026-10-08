use super::{common::*, hearing, measure, values};
use crate::{error::ApiError, typed_participants::projection as people};
use application::precautionary_measures::*;
use domain::{cases::CaseId, crypto::DocumentHasher};
use serde_json::{json, Value};

fn command(value: &MeasureDecisionCommand, case: CaseId) -> Result<Value, ApiError> {
    Ok(
        json!({"case_id":case.to_string(),"operation_id":value.operation_id.to_string(),
        "decision_id":value.decision_id.to_string(),"context":expectation(&value.context),
        "values":values::decision(&value.values)?,"anchor":value.anchor.as_ref().map(anchor_ref),
        "outcome":values::outcome(&value.outcome)?}),
    )
}
fn anchor_ref(value: &MeasureDecisionAnchorRef) -> Value {
    match value {
        MeasureDecisionAnchorRef::Initial {
            hearing_id,
            revision,
            values_digest,
            submission_digest,
        } => json!({"kind":"initial","hearing_id":hearing_id.to_string(),"revision":revision.get(),
                "values_digest":values_digest.to_hex(),"submission_digest":submission_digest.to_hex()}),
        MeasureDecisionAnchorRef::Precautionary {
            hearing_id,
            revision,
            capture_digest,
        } => {
            json!({"kind":"precautionary","hearing_id":hearing_id.to_string(),"revision":revision.get(),
                "capture_digest":capture_digest.to_hex()})
        }
    }
}
fn anchor(
    value: &MeasureDecisionAnchorMaterial,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(match value {
        MeasureDecisionAnchorMaterial::Initial(value) => {
            let s = &value.snapshot;
            let mut detail = crate::hearings::exact_projection(
                (**value).clone(),
                s.case_id,
                s.id,
                Some(s.revision),
            )?;
            detail["participants"] = json!(value.participants.iter().map(|p|json!({
                "overview":people::overview(p.overview.clone()),"values_digest":p.values_digest.to_hex()
            })).collect::<Vec<_>>());
            json!({"kind":"initial","hearing":detail})
        }
        MeasureDecisionAnchorMaterial::Precautionary(value) => {
            json!({"kind":"precautionary","capture":hearing::capture(value,h)?})
        }
    })
}
fn decision(value: &MeasureDecisionCapture, h: &dyn DocumentHasher) -> Result<Value, ApiError> {
    Ok(
        json!({"case_id":value.case_id.to_string(),"operation_id":value.operation_id.to_string(),
        "decision_id":value.decision_id.to_string(),"actor":actor(&value.actor),"context":context(&value.context,h)?,
        "values":values::decision(&value.values)?,"support":support(&value.support),
        "anchor":value.anchor.as_ref().map(|v|anchor(v,h)).transpose()?,
        "recorded_at":utc(value.recorded_at)?,"capture_digest":value.capture_digest.to_hex()}),
    )
}
fn result_sources(value: &[MeasureResultSources]) -> Result<Vec<Value>, ApiError> {
    value
        .iter()
        .map(|s| Ok(json!({"id":s.id.to_string(),"sources":measure::sources(&s.sources)?})))
        .collect()
}
pub(super) fn review(
    value: &MeasureDecisionReview,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    let m = &value.material;
    let material = json!({"context":context(&m.context,h)?,"support":support(&m.support),
        "anchor":m.anchor.as_ref().map(|v|anchor(v,h)).transpose()?,
        "predecessors":m.predecessors.iter().map(measure::owned).collect::<Result<Vec<_>, _>>()?,
        "result_sources":result_sources(&m.result_sources)?});
    Ok(
        json!({"case_id":value.case_id.to_string(),"actor":actor(&value.actor),
        "command":command(&value.command,value.case_id)?,"material":material,
        "results":value.results.iter().map(measure::result).collect::<Result<Vec<_>, _>>()?,
        "submission_digest":value.submission_digest.to_hex(),"review_digest":value.review_digest.to_hex()}),
    )
}
pub(super) fn review_v2(
    value: &MeasureDecisionReviewV2,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    let m = &value.material;
    let material = json!({"context":context(&m.context,h)?,"support":support(&m.support),
        "anchor":m.anchor.as_ref().map(|v|anchor(v,h)).transpose()?,
        "predecessors":m.predecessors.iter().map(|v|measure::owned_record(v,h)).collect::<Result<Vec<_>, _>>()?,
        "result_sources":result_sources(&m.result_sources)?});
    Ok(
        json!({"case_id":value.case_id.to_string(),"actor":actor(&value.actor),
        "command":command(&value.command,value.case_id)?,"material":material,
        "results":value.results.iter().map(measure::result_v2).collect::<Result<Vec<_>, _>>()?,
        "submission_digest":value.submission_digest.to_hex(),"review_digest":value.review_digest.to_hex()}),
    )
}
fn substitutions(values: &[MeasureSubstitutionCapture]) -> Vec<Value> {
    values
        .iter()
        .map(|v| {
            json!({"effect_key":v.effect_key.to_string(),
        "predecessors":v.predecessors.iter().map(|p|json!({"previous":reference(p.previous),
            "result":reference(p.result)})).collect::<Vec<_>>(),
        "successors":v.successors.iter().copied().map(reference).collect::<Vec<_>>()})
        })
        .collect()
}
pub(super) fn group(
    value: &MeasureDecisionGroupCapture,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(
        json!({"family":"g1","review":review(&value.review,h)?,"decision":decision(&value.decision,h)?,
        "measures":value.measures.iter().map(measure::capture).collect::<Result<Vec<_>, _>>()?,
        "substitutions":substitutions(&value.substitutions),"recorded_at":utc(value.recorded_at)?,
        "capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(super) fn group_v2(
    value: &MeasureDecisionGroupCaptureV2,
    h: &dyn DocumentHasher,
) -> Result<Value, ApiError> {
    Ok(
        json!({"family":"g2","review":review_v2(&value.review,h)?,"decision":decision(&value.decision,h)?,
        "measures":value.measures.iter().map(measure::capture_v2).collect::<Result<Vec<_>, _>>()?,
        "substitutions":substitutions(&value.substitutions),"recorded_at":utc(value.recorded_at)?,
        "capture_digest":value.capture_digest.to_hex()}),
    )
}
pub(super) fn origin(value: &MeasureGroupOrigin) -> Value {
    json!({"case_id":value.case_id.to_string(),"operation_id":value.operation_id.to_string(),
        "decision_id":value.decision_id.to_string(),"submission_digest":value.submission_digest.to_hex(),
        "review_digest":value.review_digest.to_hex(),"decision_digest":value.decision_digest.to_hex(),
        "group_digest":value.group_digest.to_hex()})
}
