use application::deadline_tracking::{
    decide_tracking_change, operational_due_at, DeadlineReviewState, TrackingChange,
    TrackingDecision, TrackingDependency, TrackingDisposition, TrackingPolicy,
    TrackingReviewReason,
};
use domain::deadlines::DeadlineStatus;
use time::{OffsetDateTime, UtcOffset};

const DEPENDENCIES: [TrackingDependency; 3] = [
    TrackingDependency::Profile,
    TrackingDependency::Source,
    TrackingDependency::Calendar,
];
const POLICIES: [TrackingPolicy; 3] = [
    TrackingPolicy::Follow,
    TrackingPolicy::Fixed,
    TrackingPolicy::Undetermined,
];

fn change(dependency: TrackingDependency, policy: TrackingPolicy) -> TrackingChange {
    TrackingChange {
        dependency,
        policy,
        selected_revision: 1,
        observed_revision: 2,
        new_revision: 3,
        retired: false,
    }
}

fn expected(
    selected_revision: u32,
    observed_revision: u32,
    disposition: TrackingDisposition,
) -> TrackingDecision {
    TrackingDecision {
        selected_revision,
        observed_revision,
        disposition,
    }
}

#[test]
fn following_calendar_recalculates_with_the_new_exact_revision() {
    let result =
        decide_tracking_change(change(TrackingDependency::Calendar, TrackingPolicy::Follow))
            .unwrap();
    assert_eq!(result, expected(3, 3, TrackingDisposition::Recalculate));
}

#[test]
fn following_source_or_profile_requires_new_human_qualification() {
    for (dependency, reason) in [
        (
            TrackingDependency::Source,
            TrackingReviewReason::SourceChanged,
        ),
        (
            TrackingDependency::Profile,
            TrackingReviewReason::ProfileChanged,
        ),
    ] {
        let result = decide_tracking_change(change(dependency, TrackingPolicy::Follow)).unwrap();
        assert_eq!(
            result,
            expected(1, 3, TrackingDisposition::ReviewRequired(reason)),
        );
    }
}

#[test]
fn fixed_dependencies_preserve_the_selection_and_observe_the_new_head() {
    for dependency in DEPENDENCIES {
        let result = decide_tracking_change(change(dependency, TrackingPolicy::Fixed)).unwrap();
        assert_eq!(result, expected(1, 3, TrackingDisposition::Preserve));
    }
}

#[test]
fn undetermined_policy_never_infers_following_or_fixing_from_revision_equality() {
    for dependency in DEPENDENCIES {
        for selected_revision in [1, 2] {
            let mut input = change(dependency, TrackingPolicy::Undetermined);
            input.selected_revision = selected_revision;
            let result = decide_tracking_change(input).unwrap();
            assert_eq!(
                result,
                expected(
                    selected_revision,
                    3,
                    TrackingDisposition::ReviewRequired(TrackingReviewReason::PolicyUndetermined,),
                ),
            );
        }
    }
}

#[test]
fn a_new_retirement_requires_review_for_every_dependency_and_policy() {
    for dependency in DEPENDENCIES {
        for policy in POLICIES {
            let mut input = change(dependency, policy);
            input.retired = true;
            let result = decide_tracking_change(input).unwrap();
            assert_eq!(
                result,
                expected(
                    1,
                    3,
                    TrackingDisposition::ReviewRequired(TrackingReviewReason::DependencyRetired,),
                ),
            );
        }
    }
}

#[test]
fn old_or_repeated_events_never_regress_or_require_another_review() {
    for dependency in DEPENDENCIES {
        for policy in POLICIES {
            for new_revision in [1, 2, 3] {
                for retired in [false, true] {
                    let mut input = change(dependency, policy);
                    input.selected_revision = 2;
                    input.observed_revision = 3;
                    input.new_revision = new_revision;
                    input.retired = retired;
                    assert_eq!(
                        decide_tracking_change(input).unwrap(),
                        expected(2, 3, TrackingDisposition::Unchanged),
                    );
                }
            }
        }
    }
}

#[test]
fn a_followed_calendar_does_not_recalculate_the_same_change_twice() {
    let first =
        decide_tracking_change(change(TrackingDependency::Calendar, TrackingPolicy::Follow))
            .unwrap();
    let mut repeated = change(TrackingDependency::Calendar, TrackingPolicy::Follow);
    repeated.selected_revision = first.selected_revision;
    repeated.observed_revision = first.observed_revision;
    assert_eq!(
        decide_tracking_change(repeated).unwrap(),
        expected(3, 3, TrackingDisposition::Unchanged),
    );
}

#[test]
fn positive_revision_gaps_are_observations_not_implicit_arithmetic_steps() {
    let mut input = change(TrackingDependency::Calendar, TrackingPolicy::Follow);
    input.new_revision = u32::MAX;
    assert_eq!(
        decide_tracking_change(input).unwrap(),
        expected(u32::MAX, u32::MAX, TrackingDisposition::Recalculate),
    );
    let mut repeated = change(TrackingDependency::Source, TrackingPolicy::Follow);
    repeated.selected_revision = u32::MAX;
    repeated.observed_revision = u32::MAX;
    repeated.new_revision = u32::MAX;
    assert_eq!(
        decide_tracking_change(repeated).unwrap(),
        expected(u32::MAX, u32::MAX, TrackingDisposition::Unchanged),
    );
}

#[test]
fn zero_revisions_are_rejected_before_stale_event_shortcuts() {
    for dependency in DEPENDENCIES {
        for policy in POLICIES {
            for (selected, observed, new) in [(0, 2, 3), (1, 0, 3), (1, 2, 0)] {
                for retired in [false, true] {
                    let mut input = change(dependency, policy);
                    input.selected_revision = selected;
                    input.observed_revision = observed;
                    input.new_revision = new;
                    input.retired = retired;
                    assert!(decide_tracking_change(input).is_err());
                }
            }
        }
    }
}

#[test]
fn selection_after_its_observed_head_is_inconsistent_even_for_an_old_event() {
    for dependency in DEPENDENCIES {
        for policy in POLICIES {
            for new_revision in [1, 2, 4] {
                let mut input = change(dependency, policy);
                input.selected_revision = 3;
                input.observed_revision = 2;
                input.new_revision = new_revision;
                assert!(decide_tracking_change(input).is_err());
            }
        }
    }
}

fn captured_due() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp_nanos(1_735_689_600_123_456_789)
        .unwrap()
        .to_offset(UtcOffset::from_hms(-6, 0, 0).unwrap())
}

#[test]
fn only_an_accepted_active_capture_can_supply_an_operational_due() {
    let captured = captured_due();
    let result = operational_due_at(
        Some(captured),
        DeadlineStatus::Active,
        DeadlineReviewState::Accepted,
    );
    assert_eq!(result, Some(captured));
    assert_eq!(result.unwrap().offset(), captured.offset());
    assert_eq!(result.unwrap().nanosecond(), 123_456_789);
}

#[test]
fn pending_or_legacy_undeclared_review_never_reactivates_historical_due() {
    let historical = captured_due();
    for status in [DeadlineStatus::Active, DeadlineStatus::Retired] {
        for review in [
            DeadlineReviewState::Pending,
            DeadlineReviewState::LegacyUndeclared,
        ] {
            assert_eq!(operational_due_at(Some(historical), status, review), None);
        }
    }
    assert_eq!(historical.nanosecond(), 123_456_789);
}

#[test]
fn retirement_blocks_operational_due_even_after_explicit_acceptance() {
    assert_eq!(
        operational_due_at(
            Some(captured_due()),
            DeadlineStatus::Retired,
            DeadlineReviewState::Accepted,
        ),
        None,
    );
}

#[test]
fn acceptance_never_fabricates_a_due_when_the_calculation_has_none() {
    for status in [DeadlineStatus::Active, DeadlineStatus::Retired] {
        for review in [
            DeadlineReviewState::Accepted,
            DeadlineReviewState::Pending,
            DeadlineReviewState::LegacyUndeclared,
        ] {
            assert_eq!(operational_due_at(None, status, review), None);
        }
    }
}
