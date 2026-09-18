#![allow(dead_code)]
use domain::judicial_calendars::CivilDate;

pub fn u32v(bytes: &mut Vec<u8>, n: u32) {
    bytes.extend(n.to_be_bytes());
}
pub fn date(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend(
        value
            .parse::<CivilDate>()
            .unwrap()
            .days_since_epoch()
            .to_be_bytes(),
    );
}
pub fn instant(bytes: &mut Vec<u8>, seconds: i64, nanos: u32, offset: i32) {
    bytes.extend(seconds.to_be_bytes());
    u32v(bytes, nanos);
    bytes.extend(offset.to_be_bytes());
}
pub fn declared() -> Vec<u8> {
    vec![1, 7, 234, 1, 6, 0]
}
pub fn natural_rule() -> Vec<u8> {
    vec![0, 0, 0, 0, 2, 0, 0, 0]
}
pub fn civil_outcome(value: &str) -> Vec<u8> {
    let mut bytes = vec![0];
    date(&mut bytes, value);
    bytes
}
pub fn natural_trace(value: &str) -> Vec<u8> {
    let mut bytes = vec![0];
    date(&mut bytes, "2026-01-06");
    u32v(&mut bytes, 2);
    bytes.push(1);
    date(&mut bytes, value);
    bytes
}
pub fn arithmetic(rule: &[u8], anchor: &[u8], outcome: &[u8], steps: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = rule.to_vec();
    bytes.extend(anchor);
    bytes.extend(outcome);
    u32v(&mut bytes, steps.len() as u32);
    for step in steps {
        bytes.extend(step);
    }
    bytes
}
/// DRES1: requirement, trigger outcome, optional rule/arithmetic/due, block list.
pub fn envelope(
    trigger: &[u8],
    rule: Option<&[u8]>,
    arithmetic: Option<&[u8]>,
    due: Option<&[u8]>,
    blocks: &[Vec<u8>],
) -> Vec<u8> {
    let mut bytes = b"DRES1".to_vec();
    bytes.extend([0, 0]);
    bytes.extend(trigger);
    for optional in [rule, arithmetic, due] {
        bytes.push(u8::from(optional.is_some()));
        if let Some(value) = optional {
            bytes.extend(value);
        }
    }
    u32v(&mut bytes, blocks.len() as u32);
    for block in blocks {
        bytes.extend(block);
    }
    bytes
}
pub fn record(blocks: &[Vec<u8>], candidate: &str) -> Vec<u8> {
    let mut trigger = vec![0];
    trigger.extend(declared());
    let rule = natural_rule();
    let arith = arithmetic(
        &rule,
        &declared(),
        &civil_outcome(candidate),
        &[natural_trace(candidate)],
    );
    envelope(&trigger, Some(&rule), Some(&arith), None, blocks)
}
pub fn assert_rejected(bytes: &[u8]) {
    assert!(application::deadline_evaluations::decode_deadline_evaluation_record(bytes).is_err());
}
