use super::{primitives::write_date, DeadlineDayCountRecord};
use domain::{deadline_days::CivilDayCountOutcome, judicial_calendars::JudicialCalendarDayOrigin};

pub(super) fn write(bytes: &mut Vec<u8>, value: &DeadlineDayCountRecord) {
    write_date(bytes, value.first_included);
    bytes.extend(value.quantity.get().to_be_bytes());
    let (tag, date) = match value.outcome {
        CivilDayCountOutcome::Candidate { date } => (0, date),
        CivilDayCountOutcome::Unresolved { date } => (1, date),
        CivilDayCountOutcome::OutsideCoverage { date } => (2, date),
        CivilDayCountOutcome::DateRangeExhausted { after } => (3, after),
    };
    bytes.push(tag);
    write_date(bytes, date);
    bytes.extend((value.trace.len() as u32).to_be_bytes());
    for step in &value.trace {
        let day = &step.day;
        write_date(bytes, day.date);
        match day.origin {
            None => bytes.push(0),
            Some(JudicialCalendarDayOrigin::WeeklyPattern(weekday)) => bytes.extend([1, weekday]),
            Some(JudicialCalendarDayOrigin::Exception(id)) => {
                bytes.push(2);
                bytes.extend(id.as_bytes());
            }
        }
        bytes.push(u8::from(day.classification.is_some()));
        if let Some(classification) = day.classification {
            bytes.push(classification.tag());
            bytes.extend((day.source_ids.len() as u32).to_be_bytes());
            for id in &day.source_ids {
                bytes.extend(id.as_bytes());
            }
            let explanation = day.explanation.as_deref().unwrap_or_default();
            bytes.extend((explanation.len() as u32).to_be_bytes());
            bytes.extend(explanation.as_bytes());
        }
        bytes.extend(step.accumulated.to_be_bytes());
    }
}
