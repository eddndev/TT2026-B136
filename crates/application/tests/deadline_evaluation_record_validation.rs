#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_evaluation_record_support;
mod deadline_evaluation_support;

use application::deadline_evaluations::*;
use deadline_evaluation_record_support::*;
use domain::{deadline_arithmetic::ArithmeticBlock, procedural_time::DeclaredProceduralTime};

fn civil(steps: &[Vec<u8>], candidate: &str, final_day: u8) -> Vec<u8> {
    let mut rule = natural_rule();
    rule[7] = final_day;
    result(&rule, &civil_outcome(candidate), steps, &[vec![10]])
}
fn result(rule: &[u8], outcome: &[u8], steps: &[Vec<u8>], blocks: &[Vec<u8>]) -> Vec<u8> {
    let mut trigger = vec![0];
    trigger.extend(declared());
    envelope(
        &trigger,
        Some(rule),
        Some(&arithmetic(rule, &declared(), outcome, steps)),
        None,
        blocks,
    )
}
fn month_trace(candidate: &str) -> Vec<u8> {
    let mut step = vec![1];
    date(&mut step, "2026-01-06");
    u32v(&mut step, 2);
    u32v(&mut step, 2026);
    step.extend([3, 6, 1]);
    date(&mut step, candidate);
    step
}
fn count_trace(tag: u8, first: &str, quantity: u32) -> Vec<u8> {
    let first: domain::judicial_calendars::CivilDate = first.parse().unwrap();
    let last = domain::judicial_calendars::CivilDate::from_days_since_epoch(
        first.days_since_epoch() + quantity as i32 - 1,
    )
    .unwrap();
    let mut step = vec![tag];
    step.extend(first.days_since_epoch().to_be_bytes());
    u32v(&mut step, quantity);
    step.push(0);
    step.extend(last.days_since_epoch().to_be_bytes());
    u32v(&mut step, quantity);
    for index in 0..quantity {
        step.extend((first.days_since_epoch() + index as i32).to_be_bytes());
        step.push(2);
        step.extend([0; 16]);
        step.extend([1, 0]);
        u32v(&mut step, 1);
        step.extend([0; 16]);
        u32v(&mut step, 8);
        step.extend(b"Declared");
        u32v(&mut step, index + 1);
    }
    step
}

#[test]
fn terminal_natural_candidate_must_match_the_result_without_recalculating_it() {
    assert_rejected(&civil(&[natural_trace("2026-01-07")], "2026-01-09", 0));
    let coherent = civil(&[natural_trace("2026-01-09")], "2026-01-09", 0);
    let value = decode_deadline_evaluation_record(&coherent).unwrap();
    assert_eq!(deadline_evaluation_record_bytes(&value), coherent);
}

#[test]
fn trace_family_and_quantity_must_match_the_captured_rule() {
    assert_rejected(&civil(&[month_trace("2026-01-07")], "2026-01-07", 0));
    let mut step = natural_trace("2026-01-07");
    step[5..9].copy_from_slice(&1_u32.to_be_bytes());
    assert_rejected(&civil(&[step], "2026-01-07", 0));
    let rule = [0, 0, 0, 0, 2, 0, 1, 0];
    assert_rejected(&result(
        &rule,
        &civil_outcome("2026-01-06"),
        &[count_trace(3, "2026-01-06", 1)],
        &[vec![10]],
    ));
}

#[test]
fn successful_arithmetic_requires_a_terminal_trace() {
    assert_rejected(&civil(&[], "2026-01-07", 0));
}

#[test]
fn monthly_candidate_must_equal_the_terminal_month_trace_candidate() {
    let rule = [1, 0, 0, 0, 2, 0];
    assert_rejected(&result(
        &rule,
        &civil_outcome("2026-03-07"),
        &[month_trace("2026-03-06")],
        &[vec![10]],
    ));
}

#[test]
fn hourly_candidate_and_quantity_must_match_the_terminal_trace() {
    let rule = [2, 0, 0, 0, 1];
    let mut candidate = vec![];
    instant(&mut candidate, 7200, 0, 0);
    let mut outcome = vec![1];
    outcome.extend(candidate);
    let mut step = vec![2];
    instant(&mut step, 0, 0, 0);
    u32v(&mut step, 1);
    step.push(1);
    instant(&mut step, 3600, 0, 0);
    assert_rejected(&result(&rule, &outcome, &[step.clone()], &[vec![0]]));
    step[17..21].copy_from_slice(&2_u32.to_be_bytes());
    let mut coherent_outcome = vec![1];
    instant(&mut coherent_outcome, 3600, 0, 0);
    assert_rejected(&result(&rule, &coherent_outcome, &[step], &[vec![0]]));
}

#[test]
fn final_day_must_follow_the_first_candidate_under_the_requested_policy() {
    let first = natural_trace("2026-01-07");
    let final_day = count_trace(4, "2026-01-07", 1);
    assert_rejected(&civil(&[first.clone(), final_day.clone()], "2026-01-08", 1));
    assert_rejected(&civil(
        &[first.clone(), count_trace(4, "2026-01-08", 1)],
        "2026-01-08",
        1,
    ));
    assert_rejected(&civil(
        &[first.clone(), count_trace(4, "2026-01-07", 2)],
        "2026-01-08",
        1,
    ));
    assert_rejected(&civil(&[first.clone(), final_day.clone()], "2026-01-07", 0));
    let valid = civil(&[first, final_day], "2026-01-07", 1);
    assert!(decode_deadline_evaluation_record(&valid).is_ok());
}

#[test]
fn cutoff_block_repeats_the_exact_civil_candidate() {
    let mut block = vec![11];
    date(&mut block, "9999-12-31");
    assert_rejected(&record(&[block], "2026-01-07"));
    let mut block = vec![11];
    date(&mut block, "2026-01-07");
    assert!(decode_deadline_evaluation_record(&record(&[block], "2026-01-07")).is_ok());
}

#[test]
fn actual_early_failures_roundtrip_without_a_trace() {
    use deadline_evaluation_support as support;
    for at in [
        DeclaredProceduralTime::unknown(),
        support::inputs::date("2026-01-06"),
        DeclaredProceduralTime::minute("2026-01-06".parse().unwrap(), 12, 30, None).unwrap(),
        DeclaredProceduralTime::second("2026-01-06".parse().unwrap(), 12, 30, 45, None).unwrap(),
    ] {
        let (mut profile, mut input, material) = support::fixture();
        support::hourly(&mut profile, &mut input);
        input.selection.qualification.as_mut().unwrap().at = at;
        let captured =
            DeadlineEvaluationRecord::capture(&support::evaluate(profile, &input, &material));
        assert!(captured.arithmetic().unwrap().trace().is_empty());
        let bytes = deadline_evaluation_record_bytes(&captured);
        assert_eq!(decode_deadline_evaluation_record(&bytes).unwrap(), captured);
    }
    let (mut profile, mut input, _) = support::fixture();
    profile.template = domain::deadline_profiles::DeadlineRuleTemplate::Fixed(
        domain::deadline_arithmetic::ArithmeticRule::Days {
            quantity: support::n(1),
            inclusion: domain::deadline_arithmetic::DayInclusion::AfterAnchor,
            basis: domain::deadline_arithmetic::DayBasis::Natural,
            final_day: domain::deadline_arithmetic::FinalDayPolicy::Preserve,
        },
    );
    let source = application::deadline_inputs::DeadlineSourceDetail::Fact(Box::new(
        support::inputs::resolution(1, false, "9999-12-31"),
    ));
    input.selection = support::inputs::request(&source).trigger;
    let material = support::inputs::material(source);
    let captured =
        DeadlineEvaluationRecord::capture(&support::evaluate(profile, &input, &material));
    assert!(captured.arithmetic().unwrap().trace().is_empty());
    assert_eq!(
        captured.blocks(),
        &[DeadlineEvaluationBlock::Arithmetic(
            ArithmeticBlock::DateRangeExhausted
        )]
    );
    let bytes = deadline_evaluation_record_bytes(&captured);
    assert_eq!(decode_deadline_evaluation_record(&bytes).unwrap(), captured);
}
