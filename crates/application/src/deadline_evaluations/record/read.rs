use super::{
    invalid, primitives::*, read_blocks, read_days, DeadlineArithmeticRecord,
    DeadlineEvaluationRecord, DeadlineTraceRecord, Reader,
};
use crate::{deadline_inputs::encoding::read as input, ApplicationError};
use domain::deadline_triggers::TriggerOutcome;

pub(super) fn decode(
    reader: &mut Reader<'_>,
) -> Result<DeadlineEvaluationRecord, ApplicationError> {
    let requirement = input::requirement(reader)?;
    let trigger_outcome = match reader.byte()? {
        0 => TriggerOutcome::Extracted {
            at: input::declared_time(reader)?,
        },
        1 => TriggerOutcome::Blocked(read_blocks::trigger(reader)?),
        _ => return Err(invalid("unknown trigger outcome")),
    };
    let rule = if reader.flag()? {
        Some(input::rule(reader)?)
    } else {
        None
    };
    let arithmetic = if reader.flag()? {
        Some(arithmetic(reader)?)
    } else {
        None
    };
    let due_at = if reader.flag()? {
        Some(instant(reader)?)
    } else {
        None
    };
    let length = count(reader, 20)?;
    let mut blocks = Vec::with_capacity(length);
    for _ in 0..length {
        blocks.push(read_blocks::evaluation(reader)?);
    }
    Ok(DeadlineEvaluationRecord {
        requirement,
        trigger_outcome,
        rule,
        arithmetic,
        due_at,
        blocks,
    })
}
fn arithmetic(reader: &mut Reader<'_>) -> Result<DeadlineArithmeticRecord, ApplicationError> {
    let rule = input::rule(reader)?;
    let anchor = input::declared_time(reader)?;
    let outcome = read_blocks::outcome(reader)?;
    let length = count(reader, 2)?;
    let mut trace = Vec::with_capacity(length);
    for _ in 0..length {
        trace.push(step(reader)?);
    }
    Ok(DeadlineArithmeticRecord {
        rule,
        anchor,
        outcome,
        trace,
    })
}
fn step(reader: &mut Reader<'_>) -> Result<DeadlineTraceRecord, ApplicationError> {
    Ok(match reader.byte()? {
        0 => DeadlineTraceRecord::NaturalDays {
            first_included: date(reader)?,
            quantity: quantity(reader)?,
            candidate: if reader.flag()? {
                Some(date(reader)?)
            } else {
                None
            },
        },
        1 => {
            let anchor = date(reader)?;
            let quantity = quantity(reader)?;
            let target_year = reader.u32()?;
            let target_month = reader.byte()?;
            let requested_day = reader.byte()?;
            if target_year == 0
                || !(1..=12).contains(&target_month)
                || !(1..=31).contains(&requested_day)
            {
                return Err(invalid("invalid month trace fields"));
            }
            let candidate = if reader.flag()? {
                Some(date(reader)?)
            } else {
                None
            };
            DeadlineTraceRecord::CivilMonths {
                anchor,
                quantity,
                target_year,
                target_month,
                requested_day,
                candidate,
            }
        }
        2 => DeadlineTraceRecord::ElapsedHours {
            start: instant(reader)?,
            quantity: quantity(reader)?,
            candidate: if reader.flag()? {
                Some(instant(reader)?)
            } else {
                None
            },
        },
        3 => DeadlineTraceRecord::CountedDays(read_days::read(reader)?),
        4 => DeadlineTraceRecord::FinalDay(read_days::read(reader)?),
        _ => return Err(invalid("unknown arithmetic trace step")),
    })
}
