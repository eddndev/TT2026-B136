use application::deadline_evaluations::DeadlineEvaluationBlock;
use domain::{
    deadline_arithmetic::ArithmeticBlock,
    deadline_profiles::DeadlineRuleBlock,
    deadline_triggers::{QualifiedTriggerPurpose, TriggerBlock, TriggerFamily, TriggerField},
    procedural_time::DeclaredProceduralPrecision,
};
use serde_json::{json, Value};

pub(super) fn evaluation(value: DeadlineEvaluationBlock) -> Value {
    match value {
        DeadlineEvaluationBlock::ScopeUnknown => json!({"kind":"scope_unknown"}),
        DeadlineEvaluationBlock::ScopeRejected => json!({"kind":"scope_rejected"}),
        DeadlineEvaluationBlock::IncidentUnknown => json!({"kind":"incident_unknown"}),
        DeadlineEvaluationBlock::UnresolvedIncident => json!({"kind":"unresolved_incident"}),
        DeadlineEvaluationBlock::ConditionMissing(id) => {
            json!({"kind":"condition_missing","id":id})
        }
        DeadlineEvaluationBlock::ConditionUnknown(id) => {
            json!({"kind":"condition_unknown","id":id})
        }
        DeadlineEvaluationBlock::ConditionRejected(id) => {
            json!({"kind":"condition_rejected","id":id})
        }
        DeadlineEvaluationBlock::Rule(value) => json!({"kind":"rule","block":rule(value)}),
        DeadlineEvaluationBlock::Trigger(value) => json!({"kind":"trigger","block":trigger(value)}),
        DeadlineEvaluationBlock::Arithmetic(value) => {
            json!({"kind":"arithmetic","block":arithmetic(value)})
        }
        DeadlineEvaluationBlock::CivilCutoffMissing => json!({"kind":"civil_cutoff_missing"}),
        DeadlineEvaluationBlock::CutoffOutsideCoverage { candidate } => {
            json!({"kind":"cutoff_outside_coverage","candidate":candidate.to_string()})
        }
    }
}
fn rule(value: DeadlineRuleBlock) -> Value {
    match value {
        DeadlineRuleBlock::MissingOrderedQuantity => json!({"kind":"missing_ordered_quantity"}),
        DeadlineRuleBlock::UnexpectedOrderedQuantity => {
            json!({"kind":"unexpected_ordered_quantity"})
        }
        DeadlineRuleBlock::OrderedQuantityExceedsMaximum { maximum, supplied } => {
            json!({"kind":"ordered_quantity_exceeds_maximum","maximum":maximum.get(),"supplied":supplied.get()})
        }
    }
}
pub(super) fn trigger(value: TriggerBlock) -> Value {
    match value {
        TriggerBlock::UnknownSource => json!({"kind":"unknown_source"}),
        TriggerBlock::AbsentField(value) => json!({"kind":"absent_field","field":field(value)}),
        TriggerBlock::IncompatibleFamily { expected, actual } => {
            json!({"kind":"incompatible_family","expected":family(expected),"actual":family(actual)})
        }
        TriggerBlock::MissingQualification { purpose: value } => {
            json!({"kind":"missing_qualification","purpose":purpose(value)})
        }
        TriggerBlock::QualificationMismatch { expected, actual } => {
            json!({"kind":"qualification_mismatch","expected":purpose(expected),"actual":purpose(actual)})
        }
        TriggerBlock::UnexpectedQualification => json!({"kind":"unexpected_qualification"}),
    }
}
pub(super) fn arithmetic(value: ArithmeticBlock) -> Value {
    match value {
        ArithmeticBlock::UnknownAnchor => json!({"kind":"unknown_anchor"}),
        ArithmeticBlock::InsufficientPrecision { observed } => {
            json!({"kind":"insufficient_precision","observed":match observed {
            DeclaredProceduralPrecision::Unknown=>"unknown",DeclaredProceduralPrecision::Date=>"date",
            DeclaredProceduralPrecision::Minute=>"minute",DeclaredProceduralPrecision::Second=>"second"}})
        }
        ArithmeticBlock::MissingOffset => json!({"kind":"missing_offset"}),
        ArithmeticBlock::MissingCalendar => json!({"kind":"missing_calendar"}),
        ArithmeticBlock::DateRangeExhausted => json!({"kind":"date_range_exhausted"}),
        ArithmeticBlock::MissingHomologousDay {
            year,
            month,
            requested_day,
        } => {
            json!({"kind":"missing_homologous_day","year":year,"month":month,"requested_day":requested_day})
        }
        ArithmeticBlock::UnresolvedCalendarDate { date } => {
            json!({"kind":"unresolved_calendar_date","date":date.to_string()})
        }
        ArithmeticBlock::OutsideCalendarCoverage { date } => {
            json!({"kind":"outside_calendar_coverage","date":date.to_string()})
        }
    }
}
fn field(value: TriggerField) -> &'static str {
    match value {
        TriggerField::ResolutionIssuedAt => "resolution_issued_at",
        TriggerField::NotificationPracticedAt => "notification_practiced_at",
        TriggerField::NotificationReceivedAt => "notification_received_at",
        TriggerField::NotificationStatedEffectAt => "notification_stated_effect_at",
        TriggerField::HearingSessionEventTime => "hearing_session_event_time",
    }
}
fn family(value: TriggerFamily) -> &'static str {
    match value {
        TriggerFamily::Resolution => "resolution",
        TriggerFamily::Notification => "notification",
        TriggerFamily::HearingResult => "hearing_result",
    }
}
fn purpose(value: QualifiedTriggerPurpose) -> &'static str {
    match value {
        QualifiedTriggerPurpose::HearingEnd => "hearing_end",
        QualifiedTriggerPurpose::OrderedPeriodStart => "ordered_period_start",
    }
}
