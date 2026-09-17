use super::{date, id, input, label};
use application::deadline_profiles::{
    DeadlineCivilCutoff, DeadlineCompletionPolicy, DeadlineProfileDefinition,
};
use time::{Time, UtcOffset};

fn cutoff(
    time: Time,
    offset: UtcOffset,
    from: &str,
    through: &str,
) -> Result<DeadlineCivilCutoff, application::deadline_profiles::DeadlineProfileError> {
    DeadlineCivilCutoff::new(
        time,
        offset,
        date(from),
        date(through),
        label("Synthetic filing channel"),
        id(0),
    )
}

#[test]
fn an_exact_civil_cutoff_retains_its_own_time_offset_channel_and_reference() {
    let time = Time::from_hms(18, 30, 7).unwrap();
    let offset = UtcOffset::from_hms(-6, 0, 0).unwrap();
    let cutoff = cutoff(time, offset, "2026-01-01", "2026-12-31").unwrap();
    assert_eq!(cutoff.time(), time);
    assert_eq!(cutoff.offset(), offset);
    assert_eq!(cutoff.from(), date("2026-01-01"));
    assert_eq!(cutoff.through(), date("2026-12-31"));
    assert_eq!(cutoff.channel(), &label("Synthetic filing channel"));
    assert_eq!(cutoff.reference_id(), id(0));
    let mut input = input();
    input.completion = DeadlineCompletionPolicy::CivilCutoff(cutoff.clone());
    let profile = DeadlineProfileDefinition::new(input).unwrap();
    assert_eq!(
        profile.completion(),
        &DeadlineCompletionPolicy::CivilCutoff(cutoff)
    );
}

#[test]
fn cutoff_rejects_subseconds_instead_of_truncating_them() {
    let time = Time::from_hms_nano(18, 0, 0, 1).unwrap();
    assert!(cutoff(time, UtcOffset::UTC, "2026-01-01", "2026-01-01").is_err());
}

#[test]
fn cutoff_offset_requires_whole_minutes_within_fourteen_hours() {
    for seconds in [-50_400, 50_400, 0] {
        assert!(cutoff(
            Time::MIDNIGHT,
            UtcOffset::from_whole_seconds(seconds).unwrap(),
            "2026-01-01",
            "2026-01-01"
        )
        .is_ok());
    }
    for seconds in [-50_460, 50_460, -1, 1] {
        assert!(cutoff(
            Time::MIDNIGHT,
            UtcOffset::from_whole_seconds(seconds).unwrap(),
            "2026-01-01",
            "2026-01-01"
        )
        .is_err());
    }
}

#[test]
fn cutoff_coverage_is_inclusive_bounded_and_never_reordered() {
    for (from, through, valid) in [
        ("2026-01-01", "2026-01-01", true),
        ("2024-01-01", "2026-12-31", true),
        ("2024-01-01", "2027-01-01", false),
        ("2026-01-02", "2026-01-01", false),
    ] {
        assert_eq!(
            cutoff(Time::MIDNIGHT, UtcOffset::UTC, from, through).is_ok(),
            valid
        );
    }
}

#[test]
fn the_first_and_last_cutoff_instants_must_remain_in_the_utc_year_range() {
    let positive = UtcOffset::from_hms(0, 1, 0).unwrap();
    let negative = UtcOffset::from_hms(0, -1, 0).unwrap();
    assert!(cutoff(Time::MIDNIGHT, positive, "0001-01-01", "0001-01-02").is_err());
    assert!(cutoff(
        Time::from_hms(0, 1, 0).unwrap(),
        positive,
        "0001-01-01",
        "0001-01-02"
    )
    .is_ok());
    assert!(cutoff(
        Time::from_hms(23, 59, 59).unwrap(),
        negative,
        "9999-12-30",
        "9999-12-31"
    )
    .is_err());
    assert!(cutoff(
        Time::from_hms(23, 58, 59).unwrap(),
        negative,
        "9999-12-30",
        "9999-12-31"
    )
    .is_ok());
}

#[test]
fn the_cutoff_reference_must_be_declared_by_the_definition() {
    let mut input = input();
    input.completion = DeadlineCompletionPolicy::CivilCutoff(
        DeadlineCivilCutoff::new(
            Time::MIDNIGHT,
            UtcOffset::UTC,
            date("2026-01-01"),
            date("2026-12-31"),
            label("Synthetic channel"),
            id(99),
        )
        .unwrap(),
    );
    assert!(DeadlineProfileDefinition::new(input).is_err());
}

#[test]
fn a_candidate_outside_cutoff_coverage_still_validates_the_arithmetic_corpus() {
    let mut input = input();
    input.completion = DeadlineCompletionPolicy::CivilCutoff(
        cutoff(Time::MIDNIGHT, UtcOffset::UTC, "2027-01-01", "2027-12-31").unwrap(),
    );
    assert!(DeadlineProfileDefinition::new(input).is_ok());
}

#[test]
fn elapsed_hours_reject_a_civil_cutoff_with_an_otherwise_valid_example() {
    use application::deadline_profiles::DeadlineExampleExpected;
    use domain::{
        deadline_arithmetic::{ArithmeticOutcome, ArithmeticRule},
        deadline_profiles::DeadlineRuleTemplate,
        procedural_time::DeclaredProceduralTime,
    };
    let mut input = input();
    input.template = DeadlineRuleTemplate::Fixed(ArithmeticRule::ElapsedHours {
        quantity: super::quantity(1),
    });
    input.completion = DeadlineCompletionPolicy::CivilCutoff(
        cutoff(Time::MIDNIGHT, UtcOffset::UTC, "2026-01-01", "2026-12-31").unwrap(),
    );
    input.examples[0].anchor =
        DeclaredProceduralTime::second(date("2026-01-06"), 0, 0, 0, Some(UtcOffset::UTC)).unwrap();
    input.examples[0].expected =
        DeadlineExampleExpected::Arithmetic(ArithmeticOutcome::InstantCandidate {
            instant: time::macros::datetime!(2026-01-06 01:00 UTC),
        });
    assert!(DeadlineProfileDefinition::new(input).is_err());
}
