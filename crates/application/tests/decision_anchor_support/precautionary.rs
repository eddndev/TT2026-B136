use crate::{decision_anchor_support::*, decision_support::*, history_support as hs};
use application::precautionary_hearings::*;
use domain::crypto::Sha256Digest;
use domain::precautionary_hearings::{PrecautionaryHearingId, PrecautionaryHearingRevision};
use time::Duration;
use uuid::Uuid;

#[test]
fn imposition_and_cancelled_precautionary_captures_are_exact_historical_anchors() {
    let scheduled = crate::precautionary_receipt_support::scheduled();
    let cancelled = crate::precautionary_receipt_support::Fixture::cancel(&scheduled)
        .capture(Some(&scheduled), at() + Duration::seconds(2));
    for hearing in [scheduled, cancelled] {
        let mut fixture = fresh_no_change(3);
        attach_precautionary(&mut fixture, hearing.clone());
        let group = capture(fixture, &empty(), at() + Duration::seconds(3));
        assert_eq!(
            group.decision.anchor,
            Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
                hearing
            )))
        );
        measure_decision_group_with_history_matches(&Hasher, &group, &empty()).unwrap();
    }
}

#[test]
fn review_anchor_requires_real_target_ancestry_even_for_no_measure_change() {
    let prior = hs::root_fixture(1, 70).capture();
    let evidence = hs::history(vec![hs::entry(&prior, &empty())]);
    let hearing = review_hearing(
        1,
        &[reference(&prior.measures[0])],
        &evidence,
        at() + Duration::seconds(1),
    );
    let mut fixture = fresh_no_change(3);
    attach_precautionary(&mut fixture, hearing);
    assert!(prepare(fixture.clone(), &empty()).is_err());
    let group = capture(fixture, &evidence, at() + Duration::seconds(2));
    assert!(group.measures.is_empty());
    measure_decision_group_with_history_matches(&Hasher, &group, &evidence).unwrap();
    measure_group_origin(&Hasher, &group, &evidence).unwrap();
}

#[test]
fn precautionary_anchor_requires_exact_family_identity_revision_and_capture_digest() {
    let hearing = crate::precautionary_receipt_support::scheduled();
    for mutation in 0..3 {
        let mut fixture = fresh_no_change(3);
        attach_precautionary(&mut fixture, hearing.clone());
        let Some(MeasureDecisionAnchorRef::Precautionary {
            hearing_id,
            revision,
            capture_digest,
        }) = &mut fixture.command.anchor
        else {
            panic!("precautionary anchor expected")
        };
        match mutation {
            0 => *hearing_id = PrecautionaryHearingId::from_uuid(Uuid::from_u128(99)),
            1 => *revision = PrecautionaryHearingRevision::new(2).unwrap(),
            _ => *capture_digest = Sha256Digest::from_array([99; 32]),
        }
        assert!(prepare(fixture, &empty()).is_err());
    }
    let mut fixture = fresh_no_change(3);
    attach_initial(&mut fixture, ordinary_initial());
    fixture.material.anchor = Some(MeasureDecisionAnchorMaterial::Precautionary(Box::new(
        hearing,
    )));
    assert!(prepare(fixture, &empty()).is_err());
}

#[test]
fn each_anchor_family_rejects_missing_reference_or_material() {
    for ordinary in [false, true] {
        for omit_reference in [false, true] {
            let mut fixture = fresh_no_change(3);
            if ordinary {
                attach_initial(&mut fixture, ordinary_initial());
            } else {
                attach_precautionary(
                    &mut fixture,
                    crate::precautionary_receipt_support::scheduled(),
                );
            }
            if omit_reference {
                fixture.command.anchor = None;
            } else {
                fixture.material.anchor = None;
            }
            assert!(prepare(fixture, &empty()).is_err());
        }
    }
}

#[test]
fn precautionary_anchor_full_receipt_cannot_be_replaced_by_a_claimed_digest() {
    for mutation in 0..3 {
        let mut hearing = crate::precautionary_receipt_support::scheduled();
        match mutation {
            0 => {
                hearing.review.participants[0].overview.display_name =
                    "Invented source label".into()
            }
            1 => hearing.review.sources.support.name = "../invalid.pdf".into(),
            _ => {
                hearing.review.sources.participants[0].bound_subject =
                    hearing.review.sources.participants[1].bound_subject.clone()
            }
        }
        if mutation == 1 {
            assert!(precautionary_hearing_review_bytes(&hearing.review).is_err());
        } else {
            hearing.review.review_digest = domain::crypto::DocumentHasher::hash_bytes(
                &Hasher,
                &precautionary_hearing_review_bytes(&hearing.review).unwrap(),
            );
            hearing.capture_digest = domain::crypto::DocumentHasher::hash_bytes(
                &Hasher,
                &precautionary_hearing_capture_bytes(&hearing).unwrap(),
            );
        }
        let mut fixture = fresh_no_change(3);
        attach_precautionary(&mut fixture, hearing);
        assert!(prepare(fixture, &empty()).is_err());
    }
}

#[test]
fn precautionary_anchor_capture_sets_the_decision_clock_floor() {
    let hearing = crate::precautionary_receipt_support::Fixture::schedule()
        .capture(None, at() + Duration::seconds(5));
    let mut fixture = fresh_no_change(3);
    attach_precautionary(&mut fixture, hearing);
    assert!(prepare(fixture.clone(), &empty())
        .unwrap()
        .into_group_capture(&Hasher, at())
        .is_err());
    capture(fixture, &empty(), at() + Duration::seconds(5));
}

#[test]
fn cancelled_review_anchor_retains_and_validates_its_original_measure_targets() {
    let prior = hs::root_fixture(1, 70).capture();
    let evidence = hs::history(vec![hs::entry(&prior, &empty())]);
    let scheduled = review_hearing(
        1,
        &[reference(&prior.measures[0])],
        &evidence,
        at() + Duration::seconds(1),
    );
    let mut cancellation = crate::precautionary_receipt_support::Fixture::cancel(&scheduled);
    cancellation.command.hearing_id = scheduled.review.command.hearing_id;
    let cancelled = prepare_precautionary_hearing_with_history(
        &Hasher,
        &cancellation.actor,
        cancellation.case_id,
        cancellation.command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: cancellation.context,
            sources: cancellation.sources,
            predecessor: Some(&scheduled),
            measure_history: &evidence,
        },
    )
    .unwrap()
    .into_capture(&Hasher, at() + Duration::seconds(2))
    .unwrap();
    let mut fixture = fresh_no_change(3);
    attach_precautionary(&mut fixture, cancelled);
    capture(fixture, &evidence, at() + Duration::seconds(3));
}

#[test]
fn anchor_and_measure_cannot_supply_conflicting_copies_of_one_subject_revision() {
    let mut hearing = crate::precautionary_receipt_support::scheduled();
    hearing.review.sources.participants[1]
        .bound_subject
        .as_mut()
        .unwrap()
        .changed_by
        .email = "different bound subject recorder".into();
    hearing.review.review_digest = domain::crypto::DocumentHasher::hash_bytes(
        &Hasher,
        &precautionary_hearing_review_bytes(&hearing.review).unwrap(),
    );
    hearing.capture_digest = domain::crypto::DocumentHasher::hash_bytes(
        &Hasher,
        &precautionary_hearing_capture_bytes(&hearing).unwrap(),
    );
    precautionary_hearing_receipt_matches(&Hasher, &hearing).unwrap();
    let mut fixture = Fixture::single();
    attach_precautionary(&mut fixture, hearing);
    assert!(prepare(fixture, &empty()).is_err());
}
