use crate::{
    error::ApiError, judicial_calendars::projection as calendar, procedural_facts::values::time,
};
use application::deadline_profiles::*;
use domain::{
    deadline_arithmetic::*,
    deadline_profiles::{DeadlineRuleTemplate, OrderedDeadlineUnit},
    deadline_triggers::*,
};
use serde_json::{json, Value};
pub(super) fn scope(v: &DeadlineProfileScope) -> Value {
    match v {
        DeadlineProfileScope::Global(value) => {
            json!({"kind":"global","value":calendar::scope(value)})
        }
        DeadlineProfileScope::Case(id) => json!({"kind":"case","case_id":id.to_string()}),
    }
}
pub(super) fn definition(v: &DeadlineProfileDefinition) -> Result<Value, ApiError> {
    Ok(
        json!({"title":v.title().as_str(),"description":v.description().as_str(),"scope":scope(v.scope()),
        "references":v.references().iter().map(calendar::source).collect::<Vec<_>>(),"trigger":trigger(v.trigger()),"template":template(v.template()),"completion":completion(v.completion()),
        "conditions":v.conditions().iter().map(|v|json!({"id":v.id,"statement":v.statement.as_str(),"reference_ids":v.reference_ids})).collect::<Vec<_>>(),
        "examples":v.examples().iter().map(|v| Ok(json!({"id":v.id,"anchor":time::project(v.anchor)?,"ordered_quantity":v.ordered_quantity.map(|v|v.get()),
            "calendar":v.calendar.as_ref().map(calendar::values),"expected":super::expected::project(v.expected),"reference_ids":v.reference_ids,"locator":v.locator.as_str()}))).collect::<Result<Vec<Value>,ApiError>>()?}),
    )
}
fn trigger(v: TriggerRequirement) -> Value {
    match v {
        TriggerRequirement::SourceField(field) => {
            json!({"kind":"source_field","field":match field {
        TriggerField::ResolutionIssuedAt=>"resolution_issued_at",TriggerField::NotificationPracticedAt=>"notification_practiced_at",
        TriggerField::NotificationReceivedAt=>"notification_received_at",TriggerField::NotificationStatedEffectAt=>"notification_stated_effect_at",
        TriggerField::HearingSessionEventTime=>"hearing_session_event_time"}})
        }
        TriggerRequirement::Qualified { purpose, family } => {
            json!({"kind":"qualified","purpose":match purpose {
        QualifiedTriggerPurpose::HearingEnd=>"hearing_end",QualifiedTriggerPurpose::OrderedPeriodStart=>"ordered_period_start"},"family":match family {
        TriggerFamily::Resolution=>"resolution",TriggerFamily::Notification=>"notification",TriggerFamily::HearingResult=>"hearing_result"}})
        }
    }
}
fn template(v: DeadlineRuleTemplate) -> Value {
    match v {
        DeadlineRuleTemplate::Fixed(rule) => {
            let (unit, q) = match rule {
                ArithmeticRule::Days {
                    quantity,
                    inclusion,
                    basis,
                    final_day,
                } => (
                    OrderedDeadlineUnit::Days {
                        inclusion,
                        basis,
                        final_day,
                    },
                    quantity,
                ),
                ArithmeticRule::CivilMonths {
                    quantity,
                    final_day,
                } => (OrderedDeadlineUnit::CivilMonths { final_day }, quantity),
                ArithmeticRule::ElapsedHours { quantity } => {
                    (OrderedDeadlineUnit::ElapsedHours, quantity)
                }
            };
            let mut rule = unit_value(unit);
            rule["quantity"] = json!(q.get());
            json!({"kind":"fixed","rule":rule})
        }
        DeadlineRuleTemplate::Ordered { unit, maximum } => {
            json!({"kind":"ordered","unit":unit_value(unit),"maximum":maximum.map(|v|v.get())})
        }
    }
}
fn unit_value(v: OrderedDeadlineUnit) -> Value {
    match v {
        OrderedDeadlineUnit::Days {
            inclusion,
            basis,
            final_day,
        } => {
            json!({"kind":"days","inclusion":match inclusion {DayInclusion::OnAnchor=>"on_anchor",DayInclusion::AfterAnchor=>"after_anchor"},
        "basis":match basis {DayBasis::Natural=>"natural",DayBasis::CalendarCountable=>"calendar_countable"},"final_day":final_day_value(final_day)})
        }
        OrderedDeadlineUnit::CivilMonths { final_day } => {
            json!({"kind":"civil_months","final_day":final_day_value(final_day)})
        }
        OrderedDeadlineUnit::ElapsedHours => json!({"kind":"elapsed_hours"}),
    }
}
fn final_day_value(v: FinalDayPolicy) -> &'static str {
    match v {
        FinalDayPolicy::Preserve => "preserve",
        FinalDayPolicy::NextCountable => "next_countable",
    }
}
fn completion(v: &DeadlineCompletionPolicy) -> Value {
    match v {
        DeadlineCompletionPolicy::ArithmeticInstant => json!({"kind":"arithmetic_instant"}),
        DeadlineCompletionPolicy::CivilCandidateOnly => json!({"kind":"civil_candidate_only"}),
        DeadlineCompletionPolicy::CivilCutoff(v) => {
            json!({"kind":"civil_cutoff","time":format!("{:02}:{:02}:{:02}",v.time().hour(),v.time().minute(),v.time().second()),
        "offset_seconds":v.offset().whole_seconds(),"from":v.from().to_string(),"through":v.through().to_string(),"channel":v.channel().as_str(),"reference_id":v.reference_id()})
        }
    }
}
