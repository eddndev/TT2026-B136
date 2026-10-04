use application::cases::CaseRevision;
use application::precautionary_hearings::*;
use domain::crypto::Sha256Digest;
use domain::precautionary_hearings::{PrecautionaryHearingId, PrecautionaryHearingRevision};
use time::Duration;
use uuid::Uuid;

use crate::precautionary_receipt_support::*;
use crate::{context_support, participant_support};

#[test]
fn schedule_forbids_a_predecessor_and_later_changes_require_one() {
    let prior = scheduled();
    assert!(Fixture::schedule().prepare(Some(&prior)).is_err());
    assert!(Fixture::replace(&prior).prepare(None).is_err());
    assert!(Fixture::cancel(&prior).prepare(None).is_err());
}

#[test]
fn every_later_change_requires_the_exact_predecessor_and_a_distinct_operation() {
    let prior = scheduled();
    for fixture in [Fixture::replace(&prior), Fixture::cancel(&prior)] {
        for field in 0..4 {
            let mut changed = fixture.clone();
            match field {
                0 => {
                    changed.command.hearing_id =
                        PrecautionaryHearingId::from_uuid(Uuid::from_u128(99))
                }
                1 => changed.command.operation_id = prior.review.command.operation_id,
                _ => match &mut changed.command.change {
                    PrecautionaryHearingChange::Replace {
                        expected_revision,
                        expected_capture_digest,
                        ..
                    }
                    | PrecautionaryHearingChange::Cancel {
                        expected_revision,
                        expected_capture_digest,
                        ..
                    } => {
                        if field == 2 {
                            *expected_revision = PrecautionaryHearingRevision::new(2).unwrap();
                        } else {
                            *expected_capture_digest = Sha256Digest::from_array([99; 32]);
                        }
                    }
                    _ => unreachable!(),
                },
            }
            assert!(changed.prepare(Some(&prior)).is_err());
        }
    }
}

#[test]
fn invalid_predecessor_receipts_are_rejected_before_building_a_new_capture() {
    let mut prior = scheduled();
    prior.review.actor.email = "tampered@example.test".into();
    for fixture in [Fixture::replace(&prior), Fixture::cancel(&prior)] {
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}

#[test]
fn cancelled_capture_is_terminal_for_new_operations() {
    let initial = scheduled();
    let cancelled = Fixture::cancel(&initial).capture(Some(&initial), at() + Duration::seconds(2));
    assert!(Fixture::replace(&cancelled)
        .prepare(Some(&cancelled))
        .is_err());
    assert!(Fixture::cancel(&cancelled)
        .prepare(Some(&cancelled))
        .is_err());
}

#[test]
fn preparation_normalizes_participant_order_including_retained_cancellation_sources() {
    let expected = scheduled();
    let mut fixture = Fixture::schedule();
    fixture.sources.participants.reverse();
    let actual = fixture.capture(None, at());
    assert_eq!(actual, expected);
    let mut cancel = Fixture::cancel(&expected);
    cancel.sources.participants.reverse();
    let capture = cancel.capture(Some(&expected), at() + Duration::seconds(2));
    assert_eq!(capture.review.sources, expected.review.sources);
}

#[test]
fn cancellation_rejects_substituted_support_metadata_and_participant_provenance() {
    let prior = scheduled();
    for field in 0..4 {
        let mut fixture = Fixture::cancel(&prior);
        match field {
            0 => fixture.sources.support.name = "substituted.pdf".into(),
            1 => fixture.sources.participants.pop().map(|_| ()).unwrap(),
            2 => {
                participant_support::manual_mut(&mut fixture.sources.participants[0])
                    .changed_by
                    .email = "other@example.test".into()
            }
            _ => {
                fixture.sources.participants[1]
                    .bound_subject
                    .as_mut()
                    .unwrap()
                    .changed_at += Duration::nanoseconds(1)
            }
        }
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}

fn with_context(mut fixture: Fixture, context: PrecautionaryContext) -> Fixture {
    match &mut fixture.command.change {
        PrecautionaryHearingChange::Schedule {
            context: expected, ..
        }
        | PrecautionaryHearingChange::Replace {
            context: expected, ..
        } => *expected = expectation(&context),
        PrecautionaryHearingChange::Cancel { .. } => {}
    }
    fixture.context = context;
    fixture
}

#[test]
fn later_mutations_reject_older_or_contradictory_administration_and_stage_context() {
    let prior = with_context(Fixture::schedule(), later_context())
        .capture(None, at() + Duration::seconds(2));
    let mut contradiction = later_context().material().clone();
    contradiction.administration.changed_by.email = "contradictory@example.test".into();
    let contradiction = PrecautionaryContext::new(&Hasher, contradiction).unwrap();
    for observed in [context(), contradiction] {
        for fixture in [Fixture::replace(&prior), Fixture::cancel(&prior)] {
            assert!(with_context(fixture, observed.clone())
                .prepare(Some(&prior))
                .is_err());
        }
    }
    let material = context_support::changed(context_support::intermediate());
    let prior = with_context(
        Fixture::schedule(),
        PrecautionaryContext::new(&Hasher, material).unwrap(),
    )
    .capture(None, at());
    for fixture in [Fixture::replace(&prior), Fixture::cancel(&prior)] {
        assert!(fixture.prepare(Some(&prior)).is_err());
    }
}

#[test]
fn cancellation_can_observe_a_later_stage_without_rewriting_scheduling_context() {
    let prior = scheduled();
    let mut material = context_support::changed(context_support::intermediate());
    material.administration = later_context().material().administration.clone();
    context_support::changed_mut(&mut material).recorded_at = at() + Duration::seconds(1);
    let observed = PrecautionaryContext::new(&Hasher, material).unwrap();
    let next = with_context(Fixture::cancel(&prior), observed.clone())
        .capture(Some(&prior), at() + Duration::seconds(2));
    assert_eq!(
        next.review.scheduling_context,
        prior.review.scheduling_context
    );
    assert_eq!(next.review.observed_context, observed);
    precautionary_hearing_transition_matches(&Hasher, &prior, &next).unwrap();
}

#[test]
fn pair_validation_requires_the_supplied_exact_predecessor_and_ordered_captures() {
    let prior = scheduled();
    let another = Fixture::schedule().capture(None, at() + Duration::nanoseconds(1));
    let next = Fixture::replace(&another).capture(Some(&another), at() + Duration::seconds(2));
    precautionary_hearing_transition_matches(&Hasher, &another, &next).unwrap();
    assert!(precautionary_hearing_transition_matches(&Hasher, &prior, &next).is_err());
    assert!(precautionary_hearing_transition_matches(&Hasher, &next, &another).is_err());
    assert!(precautionary_hearing_transition_matches(&Hasher, &prior, &prior).is_err());
}

fn changed_context_with_newer_administration() -> PrecautionaryContext {
    let mut material = context_support::changed(context_support::intermediate());
    material.administration = later_context().material().administration.clone();
    PrecautionaryContext::new(&Hasher, material).unwrap()
}

#[test]
fn regression_equal_origin_administration_history_must_preserve_full_snapshots() {
    let observed = changed_context_with_newer_administration();
    let prior = with_context(Fixture::schedule(), observed.clone())
        .capture(None, at() + Duration::seconds(2));
    for changed_time in [false, true] {
        let mut material = observed.material().clone();
        if changed_time {
            material.stage_administration.changed_at -= Duration::nanoseconds(1);
        } else {
            material.stage_administration.changed_by.email = "different-origin@example.test".into();
        }
        let contradictory = PrecautionaryContext::new(&Hasher, material).unwrap();
        assert_eq!(observed.material().stage, contradictory.material().stage);
        assert_eq!(
            observed.material().administration,
            contradictory.material().administration
        );
        for fixture in [Fixture::replace(&prior), Fixture::cancel(&prior)] {
            assert!(with_context(fixture.clone(), observed.clone())
                .prepare(Some(&prior))
                .is_ok());
            assert!(with_context(fixture, contradictory.clone())
                .prepare(Some(&prior))
                .is_err());
        }
    }
}

#[test]
fn regression_observed_to_origin_administration_history_must_preserve_full_snapshots() {
    let observed = changed_context_with_newer_administration();
    let prior = with_context(Fixture::schedule(), observed.clone())
        .capture(None, at() + Duration::seconds(2));
    let mut material = context_support::changed(context_support::trial());
    material.stage_administration = observed.material().administration.clone();
    material.administration = material.stage_administration.clone();
    material.administration.revision = CaseRevision::new(3).unwrap();
    material.administration.changed_at = at() + Duration::seconds(3);
    let origin_revision = material.stage_administration.revision;
    let origin_digest = material.stage_administration.values_digest;
    let stage = context_support::changed_mut(&mut material);
    stage.administration_revision = origin_revision;
    stage.administration_digest = origin_digest;
    stage.recorded_at = at() + Duration::seconds(3);
    let consistent = PrecautionaryContext::new(&Hasher, material.clone()).unwrap();
    for changed_time in [false, true] {
        let mut changed = material.clone();
        if changed_time {
            changed.stage_administration.changed_at -= Duration::nanoseconds(1);
        } else {
            changed.stage_administration.changed_by.email = "different-origin@example.test".into();
        }
        let contradictory = PrecautionaryContext::new(&Hasher, changed).unwrap();
        assert_eq!(consistent.material().stage, contradictory.material().stage);
        assert_eq!(
            consistent.material().administration,
            contradictory.material().administration
        );
        for fixture in [Fixture::replace(&prior), Fixture::cancel(&prior)] {
            assert!(with_context(fixture.clone(), consistent.clone())
                .prepare(Some(&prior))
                .is_ok());
            assert!(with_context(fixture, contradictory.clone())
                .prepare(Some(&prior))
                .is_err());
        }
    }
}
