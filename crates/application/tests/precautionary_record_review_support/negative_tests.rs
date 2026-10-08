use crate::{precautionary_receipt_support::Fixture as HearingFixture, record_support::*};
use application::{precautionary_hearings::*, ApplicationError};
use domain::{crypto::DocumentHasher, precautionary_hearings::*};
use time::Duration;

#[path = "negative_support.rs"]
mod support;
use support::*;

#[path = "bounds_tests.rs"]
mod bounds;
#[path = "context_tests.rs"]
mod context;

#[test]
fn fresh_schedule_and_replace_reject_an_exact_entered_in_error_record() {
    let (fixture, prior) = corrected();
    let (hearing, ancestors) = review_fixture(&prior, &fixture.history);
    let scheduled = prepare(&hearing, &ancestors, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    let marked = mark(&prior, &fixture.history);
    let (invalid_schedule, evidence) = review_fixture(&marked, &ancestors);
    assert!(prepare(&invalid_schedule, &evidence, None).is_err());
    let mut replacement = HearingFixture::replace(&scheduled);
    select(&mut replacement, record_reference(&marked.records[0]));
    assert!(prepare(&replacement, &evidence, Some(&scheduled)).is_err());
}

#[test]
fn rehashed_hearing_naming_a_marked_record_rejects_receipt_origin_history_and_cancel() {
    let (fixture, prior) = corrected();
    let (hearing, ancestors) = review_fixture(&prior, &fixture.history);
    let marked = mark(&prior, &fixture.history);
    let mut forged = prepare(&hearing, &ancestors, None)
        .unwrap()
        .into_capture(&Hasher, marked.recorded_at)
        .unwrap();
    let evidence = append_administrative(&ancestors, &marked);
    let mut replacement_values = hearing.clone();
    select(
        &mut replacement_values,
        record_reference(&marked.records[0]),
    );
    let PrecautionaryHearingChange::Schedule { values, .. } = replacement_values.command.change
    else {
        unreachable!()
    };
    forged.review.resolved_values = values.clone();
    let PrecautionaryHearingChange::Schedule { values: stored, .. } =
        &mut forged.review.command.change
    else {
        unreachable!()
    };
    *stored = values;
    refresh(&mut forged);
    assert!(
        precautionary_hearing_receipt_with_record_history_matches(&Hasher, &forged, &evidence)
            .is_err()
    );
    assert!(precautionary_hearing_origin_with_record_history(&Hasher, &forged, &evidence).is_err());
    let origin = PrecautionaryHearingOrigin {
        case_id: forged.review.case_id,
        hearing_id: forged.review.command.hearing_id,
        operation_id: forged.review.command.operation_id,
        revision: forged.review.result_revision,
        submission_digest: forged.review.submission_digest,
        review_digest: forged.review.review_digest,
        capture_digest: forged.capture_digest,
    };
    assert!(precautionary_hearing_history_with_record_history_matches(
        &Hasher,
        &[forged.clone()],
        &origin,
        &evidence
    )
    .is_err());
    assert!(prepare(&HearingFixture::cancel(&forged), &evidence, Some(&forged)).is_err());
}

#[test]
fn review_requires_exact_administrative_owner_origin_and_complete_ancestor_closure() {
    let (first, prior) = corrected();
    let second = RecordFixture::next(&prior, &first.history, 1);
    let current = second.capture();
    let (hearing, evidence) = review_fixture(&current, &second.history);
    for mutation in 0..5 {
        let mut changed = evidence.clone();
        match mutation {
            0 => changed.judicial.groups.clear(),
            1 => {
                changed.administrative.remove(0);
            }
            2 => {
                changed.administrative[1].origin.capture_digest =
                    domain::crypto::Sha256Digest::from_array([99; 32])
            }
            3 => {
                changed.administrative[0].capture.records[0]
                    .result
                    .projection
                    .subject
                    .display_name = "Forged ancestor".into()
            }
            _ => changed
                .administrative
                .push(changed.administrative[0].clone()),
        }
        assert!(prepare(&hearing, &changed, None).is_err());
    }
}

#[test]
fn review_capture_floor_uses_selected_administrative_time_instead_of_last_judicial_time() {
    let (fixture, prior) = corrected();
    let (hearing, evidence) = review_fixture(&prior, &fixture.history);
    let before = prior.recorded_at - Duration::nanoseconds(1);
    assert!(before > fixture.history.judicial.groups[0].capture.recorded_at);
    assert!(prepare(&hearing, &evidence, None)
        .unwrap()
        .into_capture(&Hasher, before)
        .is_err());
    let mut forged = prepare(&hearing, &evidence, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    forged.recorded_at = before;
    refresh(&mut forged);
    assert!(
        precautionary_hearing_receipt_with_record_history_matches(&Hasher, &forged, &evidence)
            .is_err()
    );
    for replacement in [false, true] {
        let mut next = if replacement {
            HearingFixture::replace(&forged)
        } else {
            HearingFixture::cancel(&forged)
        };
        if replacement {
            select(&mut next, record_reference(&prior.records[0]));
        }
        assert!(prepare(&next, &evidence, Some(&forged)).is_err());
    }
}

#[test]
fn rehashed_replacement_cannot_introduce_an_entered_in_error_target_into_valid_hearing_history() {
    let (fixture, prior) = corrected();
    let (hearing, ancestors) = review_fixture(&prior, &fixture.history);
    let initial = prepare(&hearing, &ancestors, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    let origin =
        precautionary_hearing_origin_with_record_history(&Hasher, &initial, &ancestors).unwrap();
    let marked = mark(&prior, &fixture.history);
    let evidence = append_administrative(&ancestors, &marked);
    let mut replacement = HearingFixture::replace(&initial);
    select(&mut replacement, record_reference(&prior.records[0]));
    let mut forged = prepare(&replacement, &ancestors, Some(&initial))
        .unwrap()
        .into_capture(&Hasher, marked.recorded_at)
        .unwrap();
    select(&mut replacement, record_reference(&marked.records[0]));
    let PrecautionaryHearingChange::Replace { values, .. } = replacement.command.change else {
        unreachable!()
    };
    forged.review.resolved_values = values.clone();
    let PrecautionaryHearingChange::Replace { values: stored, .. } =
        &mut forged.review.command.change
    else {
        unreachable!()
    };
    *stored = values;
    refresh(&mut forged);
    assert!(
        precautionary_hearing_transition_with_record_history_matches(
            &Hasher, &initial, &forged, &evidence
        )
        .is_err()
    );
    assert!(precautionary_hearing_history_with_record_history_matches(
        &Hasher,
        &[initial, forged],
        &origin,
        &evidence
    )
    .is_err());
}
