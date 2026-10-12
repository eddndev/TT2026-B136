use crate::deadline_evaluation_support::*;
use application::{deadline_evaluations::*, deadline_profiles::*};
use domain::{deadline_arithmetic::*, deadline_profiles::*, typed_participants::Uuid};
use time::{Time, UtcOffset};

fn schedule(reference: Uuid) -> DeadlineWeeklyCivilCutoff {
    DeadlineWeeklyCivilCutoff::new(
        [Some(Time::from_hms(15, 0, 0).unwrap()); 4]
            .into_iter()
            .chain([Some(Time::from_hms(14, 0, 0).unwrap()), None, None])
            .collect::<Vec<_>>()
            .try_into()
            .unwrap(),
        UtcOffset::from_hms(-6, 0, 0).unwrap(),
        date("2026-01-01"),
        date("2026-12-31"),
        label("Ordinary filing desk, declared weekly hours"),
        reference,
    )
    .unwrap()
}

#[test]
fn weekly_cutoff_uses_candidate_weekday_without_changing_arithmetic_date() {
    for (quantity, expected_date, expected_hour) in [
        (3, "2026-01-08", Some(21)),
        (4, "2026-01-09", Some(20)),
        (5, "2026-01-10", None),
        (6, "2026-01-11", None),
        (7, "2026-01-12", Some(21)),
    ] {
        let (mut p, i, m) = fixture();
        p.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::Days {
            quantity: n(quantity),
            inclusion: DayInclusion::OnAnchor,
            basis: DayBasis::Natural,
            final_day: FinalDayPolicy::Preserve,
        });
        p.examples[0].expected =
            DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::CivilCandidate {
                date: date(expected_date),
            });
        p.completion = DeadlineCompletionPolicy::WeeklyCivilCutoff(schedule(id(0)));
        let e = evaluate(p, &i, &m);
        assert_eq!(
            e.arithmetic().unwrap().outcome(),
            &ArithmeticOutcome::CivilCandidate {
                date: date(expected_date)
            }
        );
        if let Some(hour) = expected_hour {
            assert!(e.blocks().is_empty());
            assert_eq!(
                e.due_at().unwrap(),
                date(expected_date)
                    .date()
                    .with_hms(hour, 0, 0)
                    .unwrap()
                    .assume_utc()
            );
        } else {
            assert!(e.due_at().is_none());
            assert_eq!(
                e.blocks(),
                &[DeadlineEvaluationBlock::CutoffOutsideCoverage {
                    candidate: date(expected_date)
                }]
            );
        }
        let record = DeadlineEvaluationRecord::capture(&e);
        assert_eq!(
            decode_deadline_evaluation_record(&deadline_evaluation_record_bytes(&record)).unwrap(),
            record
        );
    }
}

#[test]
fn weekly_cutoff_canonical_history_keeps_all_hours_and_rejects_missing_reference() {
    let mut input = definition();
    input.completion = DeadlineCompletionPolicy::WeeklyCivilCutoff(schedule(id(0)));
    let profile = DeadlineProfileDefinition::new(input.clone()).unwrap();
    let encoded = deadline_profile_definition_bytes(&profile);
    assert_eq!(
        decode_deadline_profile_definition(&encoded).unwrap(),
        profile
    );
    for end in 0..encoded.len() {
        assert!(decode_deadline_profile_definition(&encoded[..end]).is_err());
    }
    input.completion = DeadlineCompletionPolicy::WeeklyCivilCutoff(schedule(id(90)));
    assert!(DeadlineProfileDefinition::new(input).is_err());
}

#[test]
fn weekly_cutoff_requires_covered_days_and_validates_every_time() {
    for times in [
        [None; 7],
        [Some(Time::from_hms_nano(14, 0, 0, 1).unwrap()); 7],
    ] {
        assert!(DeadlineWeeklyCivilCutoff::new(
            times,
            UtcOffset::UTC,
            date("2026-01-01"),
            date("2026-12-31"),
            label("Desk"),
            id(0)
        )
        .is_err());
    }
    let cutoff = schedule(id(0));
    assert!(cutoff.at(date("2025-12-31")).is_none());
    assert!(cutoff.at(date("2027-01-01")).is_none());
}
