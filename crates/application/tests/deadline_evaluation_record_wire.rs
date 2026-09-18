mod deadline_evaluation_record_support;
use application::deadline_evaluations::*;
use deadline_evaluation_record_support::*;
use domain::{
    deadline_arithmetic::*, deadline_profiles::*, deadline_triggers::*, procedural_time::*,
};
use std::num::NonZeroU32;
use uuid::Uuid;

fn decoded(bytes: Vec<u8>) -> DeadlineEvaluationRecord {
    let value = decode_deadline_evaluation_record(&bytes).unwrap();
    assert_eq!(deadline_evaluation_record_bytes(&value), bytes);
    value
}
#[test]
fn historical_candidate_is_decoded_without_running_current_arithmetic() {
    let value = decoded(record(&[vec![10]], "2026-01-09"));
    assert_eq!(
        value.arithmetic().unwrap().outcome(),
        &ArithmeticOutcome::CivilCandidate {
            date: "2026-01-09".parse().unwrap(),
        }
    );
    assert_eq!(
        value.blocks(),
        &[DeadlineEvaluationBlock::CivilCutoffMissing]
    );
}
#[test]
fn all_applicability_and_cutoff_blocks_preserve_order_and_nil_ids() {
    let mut blocks = vec![vec![0], vec![1], vec![2], vec![3]];
    for tag in [4, 5, 6] {
        let mut b = vec![tag];
        b.extend([0; 16]);
        blocks.push(b);
    }
    blocks.push(vec![10]);
    let mut outside = vec![11];
    date(&mut outside, "2026-01-07");
    blocks.push(outside);
    let value = decoded(record(&blocks, "2026-01-07"));
    assert_eq!(
        value.blocks(),
        &[
            DeadlineEvaluationBlock::ScopeUnknown,
            DeadlineEvaluationBlock::ScopeRejected,
            DeadlineEvaluationBlock::IncidentUnknown,
            DeadlineEvaluationBlock::UnresolvedIncident,
            DeadlineEvaluationBlock::ConditionMissing(Uuid::nil()),
            DeadlineEvaluationBlock::ConditionUnknown(Uuid::nil()),
            DeadlineEvaluationBlock::ConditionRejected(Uuid::nil()),
            DeadlineEvaluationBlock::CivilCutoffMissing,
            DeadlineEvaluationBlock::CutoffOutsideCoverage {
                candidate: "2026-01-07".parse().unwrap()
            },
        ]
    );
}
#[test]
fn every_rule_block_is_decoded_with_absent_rule_and_arithmetic() {
    let mut exceeded = vec![1];
    u32v(&mut exceeded, 2);
    u32v(&mut exceeded, 3);
    for (payload, expected) in [
        (vec![0], DeadlineRuleBlock::MissingOrderedQuantity),
        (
            exceeded,
            DeadlineRuleBlock::OrderedQuantityExceedsMaximum {
                maximum: NonZeroU32::new(2).unwrap(),
                supplied: NonZeroU32::new(3).unwrap(),
            },
        ),
        (vec![2], DeadlineRuleBlock::UnexpectedOrderedQuantity),
    ] {
        let mut block = vec![7];
        block.extend(payload);
        let value = decoded(envelope(&[0, 0], None, None, None, &[block]));
        assert_eq!(value.blocks(), &[DeadlineEvaluationBlock::Rule(expected)]);
    }
}
#[test]
fn every_trigger_block_is_preserved_in_outcome_and_block_list() {
    let cases = [
        (vec![0], TriggerBlock::UnknownSource),
        (
            vec![1, 3],
            TriggerBlock::AbsentField(TriggerField::NotificationStatedEffectAt),
        ),
        (
            vec![2, 0, 2],
            TriggerBlock::IncompatibleFamily {
                expected: TriggerFamily::Resolution,
                actual: TriggerFamily::HearingResult,
            },
        ),
        (
            vec![3, 1],
            TriggerBlock::MissingQualification {
                purpose: QualifiedTriggerPurpose::OrderedPeriodStart,
            },
        ),
        (
            vec![4, 0, 1],
            TriggerBlock::QualificationMismatch {
                expected: QualifiedTriggerPurpose::HearingEnd,
                actual: QualifiedTriggerPurpose::OrderedPeriodStart,
            },
        ),
        (vec![5], TriggerBlock::UnexpectedQualification),
    ];
    for (payload, expected) in cases {
        let mut outcome = vec![1];
        outcome.extend(&payload);
        let mut block = vec![8];
        block.extend(payload);
        let value = decoded(envelope(
            &outcome,
            Some(&natural_rule()),
            None,
            None,
            &[block],
        ));
        assert_eq!(value.trigger_outcome(), &TriggerOutcome::Blocked(expected));
        assert_eq!(
            value.blocks(),
            &[DeadlineEvaluationBlock::Trigger(expected)]
        );
    }
}
#[test]
fn every_arithmetic_block_retains_payload_and_full_u32_target_year() {
    let d = "9999-12-31".parse().unwrap();
    let mut homologue = vec![5];
    u32v(&mut homologue, 357923940);
    homologue.extend([4, 31]);
    let mut unresolved = vec![6];
    date(&mut unresolved, "9999-12-31");
    let mut outside = vec![7];
    date(&mut outside, "9999-12-31");
    let mut cases = vec![
        (vec![0], ArithmeticBlock::UnknownAnchor),
        (vec![2], ArithmeticBlock::MissingOffset),
        (vec![3], ArithmeticBlock::MissingCalendar),
        (vec![4], ArithmeticBlock::DateRangeExhausted),
        (
            homologue,
            ArithmeticBlock::MissingHomologousDay {
                year: 357923940,
                month: 4,
                requested_day: 31,
            },
        ),
        (
            unresolved,
            ArithmeticBlock::UnresolvedCalendarDate { date: d },
        ),
        (
            outside,
            ArithmeticBlock::OutsideCalendarCoverage { date: d },
        ),
    ];
    for (tag, observed) in [
        (0, DeclaredProceduralPrecision::Unknown),
        (1, DeclaredProceduralPrecision::Date),
        (2, DeclaredProceduralPrecision::Minute),
        (3, DeclaredProceduralPrecision::Second),
    ] {
        cases.push((
            vec![1, tag],
            ArithmeticBlock::InsufficientPrecision { observed },
        ));
    }
    for (payload, expected) in cases {
        let mut outcome = vec![2];
        outcome.extend(&payload);
        let mut block = vec![9];
        block.extend(payload);
        let rule = natural_rule();
        let arith = arithmetic(&rule, &[0], &outcome, &[]);
        let value = decoded(envelope(&[0, 0], Some(&rule), Some(&arith), None, &[block]));
        assert_eq!(
            value.arithmetic().unwrap().outcome(),
            &ArithmeticOutcome::Blocked(expected)
        );
        assert_eq!(
            value.blocks(),
            &[DeadlineEvaluationBlock::Arithmetic(expected)]
        );
    }
}
#[test]
fn instant_fields_keep_extreme_representable_offsets_and_nanoseconds() {
    for offset in [-93599_i32, 93599] {
        let rule = [2, 0, 0, 0, 1];
        let mut start = Vec::new();
        instant(&mut start, 0, 123, offset);
        let mut end = Vec::new();
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
        let value = decoded(envelope(
            &trigger,
            Some(&rule),
            Some(&arith),
            Some(&end),
            &[],
        ));
        assert_eq!(value.due_at().unwrap().offset().whole_seconds(), offset);
        assert_eq!(value.due_at().unwrap().nanosecond(), 123);
        let ArithmeticOutcome::InstantCandidate { instant } = value.arithmetic().unwrap().outcome()
        else {
            panic!("instant");
        };
        assert_eq!(instant.offset().whole_seconds(), offset);
        let DeadlineTraceRecord::ElapsedHours {
            start, candidate, ..
        } = &value.arithmetic().unwrap().trace()[0]
        else {
            panic!("hours");
        };
        assert_eq!(start.offset().whole_seconds(), offset);
        assert_eq!(candidate.unwrap().offset().whole_seconds(), offset);
    }
}
#[test]
fn impossible_option_combinations_and_unbounded_lengths_are_rejected() {
    let rule = natural_rule();
    let outcome = civil_outcome("2026-01-07");
    let arith = arithmetic(&rule, &declared(), &outcome, &[]);
    let mut trigger = vec![0];
    trigger.extend(declared());
    let mut due = Vec::new();
    instant(&mut due, 0, 0, 0);
    assert_rejected(&envelope(&trigger, None, Some(&arith), None, &[]));
    assert_rejected(&envelope(&trigger, Some(&rule), None, Some(&due), &[]));
    assert_rejected(&envelope(
        &trigger,
        Some(&rule),
        Some(&arith),
        Some(&due),
        &[vec![0]],
    ));
    assert_rejected(&envelope(
        &[1, 0],
        Some(&rule),
        Some(&arith),
        None,
        &[vec![8, 0]],
    ));
    let mut too_many = envelope(&[1, 0], Some(&rule), None, None, &[]);
    let len = too_many.len();
    too_many[len - 4..].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_rejected(&too_many);
    let mut too_many_steps = rule.to_vec();
    too_many_steps.extend(declared());
    too_many_steps.extend(outcome);
    u32v(&mut too_many_steps, u32::MAX);
    assert_rejected(&envelope(
        &trigger,
        Some(&rule),
        Some(&too_many_steps),
        None,
        &[],
    ));
    assert_rejected(&vec![0; 3_000_001]);
}
#[test]
fn corrupt_tags_quantities_instants_and_option_bytes_are_rejected() {
    for trigger in [
        vec![2],
        vec![1, 6],
        vec![1, 1, 5],
        vec![1, 2, 3, 0],
        vec![1, 3, 2],
    ] {
        assert_rejected(&envelope(&trigger, Some(&natural_rule()), None, None, &[]));
    }
    let mut zero = natural_rule();
    zero[4] = 0;
    assert_rejected(&envelope(&[1, 0], Some(&zero), None, None, &[vec![8, 0]]));
    for (seconds, nanos, offset) in [
        (0, 1_000_000_000, 0),
        (0, 0, 93600),
        (0, 0, -93600),
        (i64::MAX, 0, 0),
    ] {
        let mut bad = Vec::new();
        instant(&mut bad, seconds, nanos, offset);
        assert_rejected(&envelope(
            &[1, 0],
            Some(&natural_rule()),
            None,
            Some(&bad),
            &[],
        ));
    }
    for block in [vec![12], vec![7, 3], vec![9, 8], vec![9, 1, 4]] {
        assert_rejected(&record(&[block], "2026-01-07"));
    }
    let mut option = record(&[vec![10]], "2026-01-07");
    option[14] = 2;
    assert_rejected(&option);
}
