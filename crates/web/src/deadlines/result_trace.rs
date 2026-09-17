use super::result::instant;
use application::deadline_evaluations::{
    DeadlineCalendarDayRecord, DeadlineDayCountRecord, DeadlineTraceRecord,
};
use domain::{
    deadline_days::CivilDayCountOutcome,
    judicial_calendars::{JudicialCalendarClassification, JudicialCalendarDayOrigin},
};
use serde_json::{json, Value};

pub(super) fn project(value: &DeadlineTraceRecord) -> Value {
    match value {
        DeadlineTraceRecord::NaturalDays {
            first_included,
            quantity,
            candidate,
        } => json!({"kind":"natural_days",
            "first_included":first_included.to_string(),"quantity":quantity.get(),"candidate":candidate.map(|date|date.to_string())}),
        DeadlineTraceRecord::CivilMonths {
            anchor,
            quantity,
            target_year,
            target_month,
            requested_day,
            candidate,
        } => json!({"kind":"civil_months",
            "anchor":anchor.to_string(),"quantity":quantity.get(),"target_year":target_year,"target_month":target_month,
            "requested_day":requested_day,"candidate":candidate.map(|date|date.to_string())}),
        DeadlineTraceRecord::ElapsedHours {
            start,
            quantity,
            candidate,
        } => json!({"kind":"elapsed_hours", "start":instant(*start),
            "quantity":quantity.get(),"candidate":candidate.map(instant)}),
        DeadlineTraceRecord::CountedDays(value) => {
            json!({"kind":"counted_days","count":count(value)})
        }
        DeadlineTraceRecord::FinalDay(value) => json!({"kind":"final_day","count":count(value)}),
    }
}
fn count(value: &DeadlineDayCountRecord) -> Value {
    let outcome = match value.outcome() {
        CivilDayCountOutcome::Candidate { date } => {
            json!({"kind":"candidate","date":date.to_string()})
        }
        CivilDayCountOutcome::Unresolved { date } => {
            json!({"kind":"unresolved","date":date.to_string()})
        }
        CivilDayCountOutcome::OutsideCoverage { date } => {
            json!({"kind":"outside_coverage","date":date.to_string()})
        }
        CivilDayCountOutcome::DateRangeExhausted { after } => {
            json!({"kind":"date_range_exhausted","after":after.to_string()})
        }
    };
    json!({"first_included":value.first_included().to_string(),"quantity":value.quantity().get(),"accumulated":value.accumulated(),
        "outcome":outcome,"trace":value.trace().iter().map(|step|json!({"day":day(step.day()),"accumulated":step.accumulated()})).collect::<Vec<_>>()})
}
fn day(value: &DeadlineCalendarDayRecord) -> Value {
    let origin = value.origin().map(|origin| match origin {
        JudicialCalendarDayOrigin::WeeklyPattern(weekday) => {
            json!({"kind":"weekly_pattern","weekday":weekday})
        }
        JudicialCalendarDayOrigin::Exception(id) => json!({"kind":"exception","id":id}),
    });
    let classification = value.classification().map(|kind| match kind {
        JudicialCalendarClassification::Countable => "countable",
        JudicialCalendarClassification::Excluded => "excluded",
        JudicialCalendarClassification::Unresolved => "unresolved",
    });
    json!({"date":value.date().to_string(),"origin":origin,"classification":classification,
        "explanation":value.explanation(),"source_ids":value.source_ids()})
}
