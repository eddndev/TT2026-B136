use crate::{error::ApiError, procedural_facts::values::time};
use application::{deadline_evaluations::*, deadlines::*};
use domain::{deadline_triggers::*, procedural_facts::FactDeclaration};
use serde_json::{json, Value};
pub(super) fn declaration<T>(v: &FactDeclaration<T>, project: impl FnOnce(&T) -> Value) -> Value {
    match v {
        FactDeclaration::Unknown(reason) => json!({"kind":"unknown","reason":reason.as_str()}),
        FactDeclaration::Known(value) => json!({"kind":"known","value":project(value)}),
    }
}
pub(super) fn source(v: &TriggerSourceRef) -> Value {
    match v {
        TriggerSourceRef::Resolution(v) => {
            json!({"family":"resolution","id":v.id.to_string(),"revision":v.revision.get()})
        }
        TriggerSourceRef::Notification {
            id,
            revision,
            resolution,
        } => {
            json!({"family":"notification","id":id.to_string(),"revision":revision.get(),"resolution":{"id":resolution.id.to_string(),"revision":resolution.revision.get()}})
        }
        TriggerSourceRef::HearingResult(v) => {
            json!({"family":"hearing_result","hearing_id":v.hearing_id.to_string(),"result_id":v.result_id.to_string(),"revision":v.revision.get(),"agreement_id":v.agreement_id.map(|id|id.to_string())})
        }
    }
}
pub(super) fn input(v: &DeadlineEvaluationInput) -> Result<Value, ApiError> {
    let qualification=v.selection.qualification.as_ref().map(|v|Ok::<_,ApiError>(json!({"purpose":match v.purpose { QualifiedTriggerPurpose::HearingEnd=>"hearing_end",QualifiedTriggerPurpose::OrderedPeriodStart=>"ordered_period_start" },"at":time::project(v.at)?,"statement":v.statement.as_str(),"locator":v.locator.as_str()}))).transpose()?;
    let q = &v.qualification;
    Ok(
        json!({"selection":{"case_id":v.selection.case_id.to_string(),"source":declaration(&v.selection.source,source),"qualification":qualification},
        "calendar":v.calendar.map(|c|json!({"id":c.id.to_string(),"revision":c.revision.get()})),"ordered_quantity":v.ordered_quantity.map(|q|q.get()),
        "qualification":{"statement":q.statement.as_str(),"locator":q.locator.as_str(),"scope_applies":declaration(&q.scope_applies,|v|json!(v)),"unresolved_incident":declaration(&q.unresolved_incident,|v|json!(v)),
        "conditions":q.conditions.iter().map(|v|json!({"id":v.id.to_string(),"applies":declaration(&v.applies,|v|json!(v)),"locator":v.locator.as_str()})).collect::<Vec<_>>()}}),
    )
}
pub(super) fn definition(v: &DeadlineDefinition) -> Result<Value, ApiError> {
    Ok(
        json!({"title":v.title.as_str(),"profile":{"id":v.profile.id.to_string(),"revision":v.profile.revision.get()},"responsible_id":v.responsible,"input":input(&v.input)?}),
    )
}
pub(super) fn attention(v: &DeadlineAttention) -> Result<Value, ApiError> {
    Ok(match v {
        DeadlineAttention::Pending => json!({"status":"pending"}),
        DeadlineAttention::Recorded {
            occurred_at,
            statement,
            locator,
        } => {
            json!({"status":"recorded","occurred_at":time::project(*occurred_at)?,"statement":statement.as_str(),"locator":locator.as_str()})
        }
    })
}
pub(super) fn command(v: &DeadlineCommand) -> Result<Value, ApiError> {
    let mut change =
        json!({"action":v.action().as_str(),"expected_revision":v.expected_revision()});
    if let Some(reason) = v.reason() {
        change["reason"] = json!(reason.as_str());
    }
    match &v.change {
        DeadlineChange::Register { definition: v }
        | DeadlineChange::Correct { definition: v, .. } => change["definition"] = definition(v)?,
        DeadlineChange::SetAttention { attention: v, .. } => change["attention"] = attention(v)?,
        DeadlineChange::Retire { .. } => {}
    }
    Ok(
        json!({"operation_id":v.operation_id.to_string(),"deadline_id":v.deadline_id.to_string(),"change":change}),
    )
}
