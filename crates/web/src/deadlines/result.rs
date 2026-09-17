//! Project a captured result without re-extracting a trigger or evaluating arithmetic.
use super::{result_blocks as blocks, result_trace};
use crate::{
    deadline_profiles::definition_projection, error::ApiError, procedural_facts::values::time,
};
use ::time::OffsetDateTime;
use application::deadline_evaluations::{DeadlineArithmeticRecord, DeadlineEvaluationRecord};
use domain::{
    deadline_arithmetic::{
        ArithmeticOutcome, ArithmeticRule, DayBasis, DayInclusion, FinalDayPolicy,
    },
    deadline_triggers::TriggerOutcome,
};
use serde_json::{json, Value};

pub(super) fn project(value: &DeadlineEvaluationRecord) -> Result<Value, ApiError> {
    let trigger = match value.trigger_outcome() {
        TriggerOutcome::Extracted { at } => json!({"kind":"extracted","at":time::project(*at)?}),
        TriggerOutcome::Blocked(cause) => json!({"kind":"blocked","block":blocks::trigger(*cause)}),
    };
    Ok(json!({
        "requirement":definition_projection::trigger(value.requirement()),
        "trigger_outcome":trigger,"rule":value.rule().map(rule),
        "arithmetic":value.arithmetic().map(arithmetic).transpose()?,
        "due_at":value.due_at().map(instant),
        "blocks":value.blocks().iter().copied().map(blocks::evaluation).collect::<Vec<_>>()
    }))
}
fn arithmetic(value: &DeadlineArithmeticRecord) -> Result<Value, ApiError> {
    Ok(
        json!({"rule":rule(value.rule()),"anchor":time::project(value.anchor())?,
        "outcome":outcome(*value.outcome()),"trace":value.trace().iter().map(result_trace::project).collect::<Vec<_>>()}),
    )
}
fn rule(value: ArithmeticRule) -> Value {
    match value {
        ArithmeticRule::Days {
            quantity,
            inclusion,
            basis,
            final_day,
        } => json!({
            "kind":"days","quantity":quantity.get(),
            "inclusion":match inclusion {DayInclusion::OnAnchor=>"on_anchor",DayInclusion::AfterAnchor=>"after_anchor"},
            "basis":match basis {DayBasis::Natural=>"natural",DayBasis::CalendarCountable=>"calendar_countable"},
            "final_day":final_day_name(final_day)}),
        ArithmeticRule::CivilMonths {
            quantity,
            final_day,
        } => {
            json!({"kind":"civil_months","quantity":quantity.get(),"final_day":final_day_name(final_day)})
        }
        ArithmeticRule::ElapsedHours { quantity } => {
            json!({"kind":"elapsed_hours","quantity":quantity.get()})
        }
    }
}
fn final_day_name(value: FinalDayPolicy) -> &'static str {
    match value {
        FinalDayPolicy::Preserve => "preserve",
        FinalDayPolicy::NextCountable => "next_countable",
    }
}
fn outcome(value: ArithmeticOutcome) -> Value {
    match value {
        ArithmeticOutcome::CivilCandidate { date } => {
            json!({"kind":"civil_candidate","date":date.to_string()})
        }
        ArithmeticOutcome::InstantCandidate { instant: at } => {
            json!({"kind":"instant_candidate","instant":instant(at)})
        }
        ArithmeticOutcome::Blocked(cause) => {
            json!({"kind":"blocked","block":blocks::arithmetic(cause)})
        }
    }
}
/// Preserve every instant component, including offsets outside RFC 3339 syntax.
pub(super) fn instant(value: OffsetDateTime) -> Value {
    json!({"unix_seconds":value.unix_timestamp(),"nanosecond":value.nanosecond(),"offset_seconds":value.offset().whole_seconds()})
}
