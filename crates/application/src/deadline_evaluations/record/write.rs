use super::{
    primitives::*, write_blocks, write_days, DeadlineArithmeticRecord, DeadlineEvaluationRecord,
    DeadlineTraceRecord,
};
use crate::deadline_inputs::encoding::write as input;
use domain::deadline_triggers::TriggerOutcome;

pub(super) fn encode(value: &DeadlineEvaluationRecord) -> Vec<u8> {
    let mut bytes = b"DRES1".to_vec();
    input::requirement(&mut bytes, value.requirement);
    match value.trigger_outcome {
        TriggerOutcome::Extracted { at } => {
            bytes.push(0);
            input::declared_time(&mut bytes, at);
        }
        TriggerOutcome::Blocked(block) => {
            bytes.push(1);
            write_blocks::trigger(&mut bytes, block);
        }
    }
    bytes.push(u8::from(value.rule.is_some()));
    if let Some(rule) = value.rule {
        input::rule(&mut bytes, rule);
    }
    bytes.push(u8::from(value.arithmetic.is_some()));
    if let Some(arithmetic) = &value.arithmetic {
        write_arithmetic(&mut bytes, arithmetic);
    }
    bytes.push(u8::from(value.due_at.is_some()));
    if let Some(due_at) = value.due_at {
        write_instant(&mut bytes, due_at);
    }
    bytes.extend((value.blocks.len() as u32).to_be_bytes());
    for block in &value.blocks {
        write_blocks::evaluation(&mut bytes, *block);
    }
    bytes
}
fn write_arithmetic(bytes: &mut Vec<u8>, value: &DeadlineArithmeticRecord) {
    input::rule(bytes, value.rule);
    input::declared_time(bytes, value.anchor);
    write_blocks::outcome(bytes, value.outcome);
    bytes.extend((value.trace.len() as u32).to_be_bytes());
    for step in &value.trace {
        match step {
            DeadlineTraceRecord::NaturalDays {
                first_included,
                quantity,
                candidate,
            } => {
                bytes.push(0);
                write_date(bytes, *first_included);
                bytes.extend(quantity.get().to_be_bytes());
                bytes.push(u8::from(candidate.is_some()));
                if let Some(candidate) = candidate {
                    write_date(bytes, *candidate);
                }
            }
            DeadlineTraceRecord::CivilMonths {
                anchor,
                quantity,
                target_year,
                target_month,
                requested_day,
                candidate,
            } => {
                bytes.push(1);
                write_date(bytes, *anchor);
                bytes.extend(quantity.get().to_be_bytes());
                bytes.extend(target_year.to_be_bytes());
                bytes.extend([*target_month, *requested_day, u8::from(candidate.is_some())]);
                if let Some(candidate) = candidate {
                    write_date(bytes, *candidate);
                }
            }
            DeadlineTraceRecord::ElapsedHours {
                start,
                quantity,
                candidate,
            } => {
                bytes.push(2);
                write_instant(bytes, *start);
                bytes.extend(quantity.get().to_be_bytes());
                bytes.push(u8::from(candidate.is_some()));
                if let Some(candidate) = candidate {
                    write_instant(bytes, *candidate);
                }
            }
            DeadlineTraceRecord::CountedDays(value) => {
                bytes.push(3);
                write_days::write(bytes, value);
            }
            DeadlineTraceRecord::FinalDay(value) => {
                bytes.push(4);
                write_days::write(bytes, value);
            }
        }
    }
}
