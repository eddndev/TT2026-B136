#![allow(dead_code)]
#[path = "../../../application/tests/deadline_evaluation_record_support/mod.rs"]
pub mod wire;
use application::deadline_evaluations::{
    decode_deadline_evaluation_record, DeadlineEvaluationRecord,
};
use serde_json::{json, Value};
pub use wire::*;
pub fn decoded(bytes: Vec<u8>) -> DeadlineEvaluationRecord {
    decode_deadline_evaluation_record(&bytes).unwrap()
}
pub fn time_json() -> Value {
    json!({"precision":"date","year":2026,"month":1,"day":6,"offset_seconds":null})
}
pub fn days_rule_json() -> Value {
    json!({"kind":"days","quantity":2,"inclusion":"on_anchor","basis":"natural","final_day":"preserve"})
}
pub fn instant_json(seconds: i64, nanos: u32, offset: i32) -> Value {
    json!({"unix_seconds":seconds,"nanosecond":nanos,"offset_seconds":offset})
}
pub fn monthly() -> DeadlineEvaluationRecord {
    let rule = [1, 0, 0, 0, 1, 0];
    let anchor = [1, 7, 234, 1, 31, 0];
    let mut cause = vec![5];
    u32v(&mut cause, 2026);
    cause.extend([2, 31]);
    let mut outcome = vec![2];
    outcome.extend(&cause);
    let mut step = vec![1];
    date(&mut step, "2026-01-31");
    u32v(&mut step, 1);
    u32v(&mut step, 2026);
    step.extend([2, 31, 0]);
    let arith = arithmetic(&rule, &anchor, &outcome, &[step]);
    let mut trigger = vec![0];
    trigger.extend(anchor);
    let mut block = vec![9];
    block.extend(cause);
    decoded(envelope(
        &trigger,
        Some(&rule),
        Some(&arith),
        None,
        &[block],
    ))
}
pub fn hourly(offset: i32) -> DeadlineEvaluationRecord {
    let rule = [2, 0, 0, 0, 1];
    let mut start = vec![];
    instant(&mut start, 0, 123, offset);
    let mut end = vec![];
    instant(&mut end, 3600, 123, offset);
    let mut outcome = vec![1];
    outcome.extend(&end);
    let mut step = vec![2];
    step.extend(start);
    u32v(&mut step, 1);
    step.push(1);
    step.extend(&end);
    let arith = arithmetic(&rule, &declared(), &outcome, &[step]);
    let mut trigger = vec![0];
    trigger.extend(declared());
    decoded(envelope(
        &trigger,
        Some(&rule),
        Some(&arith),
        Some(&end),
        &[],
    ))
}
fn count(origin: u8, outside: bool) -> Vec<u8> {
    let mut bytes = vec![];
    date(&mut bytes, "2026-01-06");
    u32v(&mut bytes, 1);
    bytes.push(if outside { 2 } else { 0 });
    date(&mut bytes, "2026-01-06");
    u32v(&mut bytes, 1);
    date(&mut bytes, "2026-01-06");
    bytes.push(origin);
    match origin {
        1 => bytes.push(2),
        2 => bytes.extend([0; 16]),
        _ => {}
    }
    bytes.push(u8::from(!outside));
    if !outside {
        bytes.push(0);
        u32v(&mut bytes, 1);
        bytes.extend([0; 16]);
        u32v(&mut bytes, 19);
        bytes.extend(b"Declared countable.");
    }
    u32v(&mut bytes, u32::from(!outside));
    bytes
}
pub fn calendar(outside: bool) -> DeadlineEvaluationRecord {
    let rule = [0, 0, 0, 0, 1, 0, 1, 1];
    let mut first = vec![3];
    first.extend(count(if outside { 0 } else { 1 }, outside));
    let mut steps = vec![first];
    if !outside {
        let mut last = vec![4];
        last.extend(count(2, false));
        steps.push(last);
    }
    let mut outcome = if outside { vec![2, 7] } else { vec![0] };
    date(&mut outcome, "2026-01-06");
    let mut block = if outside { vec![9, 7] } else { vec![10] };
    if outside {
        date(&mut block, "2026-01-06");
    }
    let arith = arithmetic(&rule, &declared(), &outcome, &steps);
    let mut trigger = vec![0];
    trigger.extend(declared());
    decoded(envelope(
        &trigger,
        Some(&rule),
        Some(&arith),
        None,
        &[block],
    ))
}
