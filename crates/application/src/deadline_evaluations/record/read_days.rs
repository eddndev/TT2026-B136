use super::{
    invalid, primitives::*, DeadlineCalendarDayRecord, DeadlineDayCountRecord,
    DeadlineDayStepRecord, Reader,
};
use crate::ApplicationError;
use domain::{
    deadline_days::CivilDayCountOutcome,
    judicial_calendars::{
        JudicialCalendarClassification as Class, JudicialCalendarDayOrigin as Origin,
        JudicialCalendarRule,
    },
};

pub(super) fn read(reader: &mut Reader<'_>) -> Result<DeadlineDayCountRecord, ApplicationError> {
    let first_included = date(reader)?;
    let quantity = quantity(reader)?;
    let tag = reader.byte()?;
    let ending = date(reader)?;
    let outcome = match tag {
        0 => CivilDayCountOutcome::Candidate { date: ending },
        1 => CivilDayCountOutcome::Unresolved { date: ending },
        2 => CivilDayCountOutcome::OutsideCoverage { date: ending },
        3 => CivilDayCountOutcome::DateRangeExhausted { after: ending },
        _ => return Err(invalid("unknown count outcome")),
    };
    let length = count(reader, 1097)?;
    if length == 0 {
        return Err(invalid("empty calendar trace"));
    }
    let mut trace = Vec::with_capacity(length);
    for _ in 0..length {
        trace.push(DeadlineDayStepRecord {
            day: day(reader)?,
            accumulated: reader.u32()?,
        });
    }
    let value = DeadlineDayCountRecord {
        first_included,
        quantity,
        outcome,
        trace,
    };
    validate(&value)?;
    Ok(value)
}
fn day(reader: &mut Reader<'_>) -> Result<DeadlineCalendarDayRecord, ApplicationError> {
    let date = date(reader)?;
    let origin = match reader.byte()? {
        0 => None,
        1 => {
            let day = reader.byte()?;
            if day != date.weekday() {
                return Err(invalid("invalid weekday origin"));
            }
            Some(Origin::WeeklyPattern(day))
        }
        2 => Some(Origin::Exception(reader.uuid()?)),
        _ => return Err(invalid("unknown day origin")),
    };
    let (classification, explanation, source_ids) = if reader.flag()? {
        let classification = match reader.byte()? {
            0 => Class::Countable,
            1 => Class::Excluded,
            2 => Class::Unresolved,
            _ => return Err(invalid("unknown calendar classification")),
        };
        let length = count(reader, 16)?;
        let mut sources = Vec::with_capacity(length);
        for _ in 0..length {
            sources.push(reader.uuid()?);
        }
        let explanation = reader.text(1024)?.to_owned();
        let rule = JudicialCalendarRule::new(classification, sources.clone(), &explanation)
            .map_err(invalid)?;
        if rule.source_ids() != sources || rule.explanation() != explanation {
            return Err(invalid("noncanonical calendar rule"));
        }
        (Some(classification), Some(explanation), sources)
    } else {
        (None, None, vec![])
    };
    if origin.is_some() != classification.is_some() {
        return Err(invalid("day origin and rule disagree"));
    }
    Ok(DeadlineCalendarDayRecord {
        date,
        origin,
        classification,
        explanation,
        source_ids,
    })
}
fn validate(value: &DeadlineDayCountRecord) -> Result<(), ApplicationError> {
    let mut accumulated = 0_u32;
    for (index, step) in value.trace.iter().enumerate() {
        if step.day.date.days_since_epoch()
            != value.first_included.days_since_epoch() + index as i32
        {
            return Err(invalid("calendar trace is not contiguous"));
        }
        if step.day.classification == Some(Class::Countable) {
            accumulated += 1;
        }
        if step.accumulated != accumulated || accumulated > value.quantity.get() {
            return Err(invalid("calendar accumulation differs"));
        }
        if index + 1 < value.trace.len()
            && (accumulated == value.quantity.get()
                || matches!(step.day.classification, None | Some(Class::Unresolved)))
        {
            return Err(invalid("calendar trace continues after stopping"));
        }
    }
    let last = value.trace.last().ok_or_else(|| invalid("empty count"))?;
    let expected = match last.day.classification {
        Some(Class::Countable) if accumulated == value.quantity.get() => {
            CivilDayCountOutcome::Candidate {
                date: last.day.date,
            }
        }
        Some(Class::Unresolved) => CivilDayCountOutcome::Unresolved {
            date: last.day.date,
        },
        None => CivilDayCountOutcome::OutsideCoverage {
            date: last.day.date,
        },
        _ if last.day.date.date().to_calendar_date() == (9999, time::Month::December, 31) => {
            CivilDayCountOutcome::DateRangeExhausted {
                after: last.day.date,
            }
        }
        _ => return Err(invalid("calendar count lacks a terminal step")),
    };
    if value.outcome != expected {
        return Err(invalid("calendar outcome and terminal step disagree"));
    }
    Ok(())
}
