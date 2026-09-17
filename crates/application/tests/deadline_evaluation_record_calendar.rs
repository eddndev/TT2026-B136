#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_evaluation_record_support;
mod deadline_evaluation_support;
use application::{deadline_evaluations::*, deadline_inputs::*};
use deadline_evaluation_record_support::{arithmetic, declared, envelope, u32v};
use deadline_evaluation_support::{evaluate, fixture, inputs, n};
use domain::{deadline_arithmetic::*, deadline_days::CivilDayCountOutcome, judicial_calendars::*};

#[test]
fn capture_keeps_counted_and_final_day_classification_provenance() {
    let (mut p, mut i, mut m) = fixture();
    let calendar = inputs::calendar(1, false, JudicialCalendarClassification::Countable);
    let rule = ArithmeticRule::Days {
        quantity: n(2),
        inclusion: DayInclusion::OnAnchor,
        basis: DayBasis::CalendarCountable,
        final_day: FinalDayPolicy::NextCountable,
    };
    p.template = domain::deadline_profiles::DeadlineRuleTemplate::Fixed(rule);
    p.examples[0].calendar = Some(calendar.values.clone());
    i.calendar = Some(DeadlineCalendarRef {
        id: calendar.id,
        revision: calendar.revision,
    });
    m.calendar = Some(calendar.clone());
    m.calendar_head = Some(calendar);
    let evaluation = evaluate(p, &i, &m);
    let captured = DeadlineEvaluationRecord::capture(&evaluation);
    let record =
        decode_deadline_evaluation_record(&deadline_evaluation_record_bytes(&captured)).unwrap();
    assert_eq!(record, captured);
    let steps = record.arithmetic().unwrap().trace();
    let DeadlineTraceRecord::CountedDays(count) = &steps[0] else {
        panic!("counted days");
    };
    let DeadlineTraceRecord::FinalDay(final_day) = &steps[1] else {
        panic!("final day");
    };
    assert_eq!(count.quantity(), n(2));
    assert_eq!(count.trace().len(), 2);
    assert_eq!(final_day.quantity(), n(1));
    assert_eq!(final_day.trace().len(), 1);
    assert_eq!(
        final_day.outcome(),
        CivilDayCountOutcome::Candidate {
            date: "2026-01-07".parse().unwrap()
        }
    );
    for step in count.trace().iter().chain(final_day.trace()) {
        let day = step.day();
        assert_eq!(
            day.classification(),
            Some(JudicialCalendarClassification::Countable)
        );
        assert_eq!(day.source_ids(), &[uuid::Uuid::from_u128(88)]);
        assert_eq!(day.explanation(), Some("Declared classification"));
        assert_eq!(
            day.origin(),
            Some(JudicialCalendarDayOrigin::WeeklyPattern(
                day.date().weekday()
            ))
        );
    }
}

fn day(
    bytes: &mut Vec<u8>,
    date: CivilDate,
    origin: u8,
    classification: u8,
    source_count: u32,
    explanation: &str,
) {
    bytes.extend(date.days_since_epoch().to_be_bytes());
    bytes.push(origin);
    match origin {
        1 => bytes.push(date.weekday()),
        2 => bytes.extend([0; 16]),
        _ => {}
    }
    bytes.push(u8::from(origin != 0));
    if origin != 0 {
        bytes.push(classification);
        u32v(bytes, source_count);
        for index in 0..source_count {
            bytes.extend(uuid::Uuid::from_u128(u128::from(index)).as_bytes());
        }
        u32v(bytes, explanation.len() as u32);
        bytes.extend(explanation.as_bytes());
    }
    u32v(bytes, u32::from(origin != 0 && classification == 0));
}
fn calendar_record(
    count_outcome: u8,
    classification: u8,
    origin: u8,
    source_count: u32,
    explanation: &str,
    length: u32,
) -> Vec<u8> {
    let first: CivilDate = (if count_outcome == 3 {
        "9999-12-31"
    } else {
        "2026-01-06"
    })
    .parse()
    .unwrap();
    let last = CivilDate::from_days_since_epoch(
        first.days_since_epoch() + length.saturating_sub(1) as i32,
    )
    .unwrap();
    let mut count = Vec::new();
    count.extend(first.days_since_epoch().to_be_bytes());
    u32v(&mut count, 1);
    count.push(count_outcome);
    count.extend(last.days_since_epoch().to_be_bytes());
    u32v(&mut count, length);
    for index in 0..length {
        let at = CivilDate::from_days_since_epoch(first.days_since_epoch() + index as i32).unwrap();
        if index + 1 == length {
            day(
                &mut count,
                at,
                origin,
                classification,
                source_count,
                explanation,
            );
        } else {
            day(&mut count, at, 2, 1, source_count, explanation);
        }
    }
    let rule = [0, 0, 0, 0, 1, 0, 1, 0];
    let mut step = vec![3];
    step.extend(count);
    let mut outcome = match count_outcome {
        0 => vec![0],
        2 => vec![2, 7],
        3 => vec![2, 4],
        _ => vec![2, 6],
    };
    if count_outcome != 3 {
        outcome.extend(last.days_since_epoch().to_be_bytes());
    }
    let anchor = if count_outcome == 3 {
        vec![1, 39, 15, 12, 31, 0]
    } else {
        declared()
    };
    let arith = arithmetic(&rule, &anchor, &outcome, &[step]);
    let mut trigger = vec![0];
    trigger.extend(anchor);
    let mut block = vec![9];
    block.extend(&outcome[1..]);
    if count_outcome == 0 {
        block = vec![10];
    }
    envelope(&trigger, Some(&rule), Some(&arith), None, &[block])
}
#[test]
fn all_count_outcomes_and_exception_nil_identity_are_lossless() {
    for (tag, expected) in [
        (
            0,
            CivilDayCountOutcome::Candidate {
                date: "2026-01-06".parse().unwrap(),
            },
        ),
        (
            1,
            CivilDayCountOutcome::Unresolved {
                date: "2026-01-06".parse().unwrap(),
            },
        ),
        (
            2,
            CivilDayCountOutcome::OutsideCoverage {
                date: "2026-01-06".parse().unwrap(),
            },
        ),
        (
            3,
            CivilDayCountOutcome::DateRangeExhausted {
                after: "9999-12-31".parse().unwrap(),
            },
        ),
    ] {
        let (classification, origin, sources) = match tag {
            0 => (0, 2, 1),
            1 => (2, 2, 0),
            2 => (2, 0, 0),
            _ => (1, 2, 1),
        };
        let bytes = calendar_record(tag, classification, origin, sources, "Declared source", 1);
        let record = decode_deadline_evaluation_record(&bytes).unwrap();
        assert_eq!(deadline_evaluation_record_bytes(&record), bytes);
        let DeadlineTraceRecord::CountedDays(count) = &record.arithmetic().unwrap().trace()[0]
        else {
            panic!("count");
        };
        assert_eq!(count.outcome(), expected);
        assert_eq!(
            count.trace()[0].day().origin(),
            (tag != 2).then_some(JudicialCalendarDayOrigin::Exception(uuid::Uuid::nil()))
        );
    }
}
#[test]
fn maximum_day_trace_source_count_and_utf8_explanation_have_explicit_bounds() {
    let explanation = "\u{1f600}".repeat(256);
    assert_eq!(explanation.len(), 1024);
    let bytes = calendar_record(2, 2, 0, 16, &explanation, 1097);
    let record = decode_deadline_evaluation_record(&bytes).unwrap();
    assert_eq!(deadline_evaluation_record_bytes(&record), bytes);
    let DeadlineTraceRecord::CountedDays(count) = &record.arithmetic().unwrap().trace()[0] else {
        panic!("count");
    };
    assert_eq!(count.trace().len(), 1097);
    assert_eq!(count.trace()[1095].day().source_ids().len(), 16);
    assert_eq!(
        count.trace()[0].day().explanation(),
        Some(explanation.as_str())
    );
    for bad in [
        calendar_record(1, 2, 2, 17, "Valid", 1),
        calendar_record(1, 2, 2, 0, &"x".repeat(257), 1),
        calendar_record(1, 2, 2, 0, "\u{1f600}".repeat(257).as_str(), 1),
        calendar_record(2, 2, 0, 16, "Valid", 1098),
    ] {
        assert!(decode_deadline_evaluation_record(&bad).is_err());
    }
}
#[test]
fn invalid_calendar_tags_unattributed_classification_and_normalized_text_are_rejected() {
    for bad in [
        calendar_record(4, 2, 2, 0, "Valid", 1),
        calendar_record(1, 3, 2, 0, "Valid", 1),
        calendar_record(1, 0, 2, 0, "Valid", 1),
        calendar_record(1, 2, 2, 0, " trailing ", 1),
        calendar_record(1, 2, 2, 0, "a\r\nb", 1),
        calendar_record(1, 2, 3, 0, "Valid", 1),
    ] {
        assert!(decode_deadline_evaluation_record(&bad).is_err());
    }
}
#[test]
fn monthly_trace_preserves_target_year_and_absent_candidate() {
    let rule = [1, 0xff, 0xff, 0xff, 0xff, 0];
    let mut step = vec![1];
    deadline_evaluation_record_support::date(&mut step, "2026-01-06");
    u32v(&mut step, u32::MAX);
    u32v(&mut step, 357915967);
    step.extend([4, 6, 0]);
    let arith = arithmetic(&rule, &declared(), &[2, 4], &[step]);
    let mut trigger = vec![0];
    trigger.extend(declared());
    let bytes = envelope(&trigger, Some(&rule), Some(&arith), None, &[vec![9, 4]]);
    let value = decode_deadline_evaluation_record(&bytes).unwrap();
    assert_eq!(deadline_evaluation_record_bytes(&value), bytes);
    assert_eq!(
        value.arithmetic().unwrap().trace(),
        &[DeadlineTraceRecord::CivilMonths {
            anchor: "2026-01-06".parse().unwrap(),
            quantity: n(u32::MAX),
            target_year: 357915967,
            target_month: 4,
            requested_day: 6,
            candidate: None,
        }]
    );
}
