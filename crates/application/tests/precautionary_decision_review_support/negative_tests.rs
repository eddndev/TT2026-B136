use crate::decision_review_support::*;
use domain::crypto::{DocumentHasher, Sha256Digest};

#[path = "negative_support.rs"]
mod support;
use support::*;
#[path = "bounds_tests.rs"]
mod bounds;
#[path = "context_tests.rs"]
mod context;
#[path = "source_tests.rs"]
mod sources;

#[test]
fn review_of_m2_requires_the_actual_g2_owner_and_every_g1_administrative_ancestor() {
    let request = judicial_fixture();
    let group = request.capture();
    let original = DecisionReviewFixture::schedule(
        vec![reference_v2(&group.measures[0])],
        append_v2(&request.history, &group),
    );
    for missing in 0..3 {
        let mut fixture = original.clone();
        match missing {
            0 => fixture.decision_history.decisions.clear(),
            1 => fixture.decision_history.records.judicial.groups.clear(),
            _ => fixture.decision_history.records.administrative.clear(),
        }
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn m2_review_binds_every_owner_origin_field_and_the_exact_member_reference() {
    let request = judicial_fixture();
    let group = request.capture();
    let selected = reference_v2(&group.measures[0]);
    let original =
        DecisionReviewFixture::schedule(vec![selected], append_v2(&request.history, &group));
    for mutation in 0..7 {
        let mut fixture = original.clone();
        let origin = &mut fixture.decision_history.decisions[0].origin;
        let other = uuid::Uuid::from_u128(999);
        let digest = Sha256Digest::from_array([99; 32]);
        match mutation {
            0 => origin.case_id = domain::cases::CaseId::from_uuid(other),
            1 => origin.operation_id = MeasureDecisionOperationId::from_uuid(other),
            2 => origin.decision_id = MeasureDecisionId::from_uuid(other),
            3 => origin.submission_digest = digest,
            4 => origin.review_digest = digest,
            5 => origin.decision_digest = digest,
            _ => origin.group_digest = digest,
        }
        assert!(fixture.prepare(None).is_err());
    }
    for target in [
        PrecautionaryMeasureRef::new(id(999), selected.revision(), selected.digest()),
        PrecautionaryMeasureRef::new(
            selected.id(),
            MeasureRevision::new(9).unwrap(),
            selected.digest(),
        ),
        PrecautionaryMeasureRef::new(
            selected.id(),
            selected.revision(),
            Sha256Digest::from_array([99; 32]),
        ),
    ] {
        let mut fixture = original.clone();
        crate::record_review_support::select_targets(&mut fixture.hearing, vec![target]);
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn reviewing_one_m2_still_requires_every_exact_sibling_in_its_owning_group() {
    let request = FixtureV2::initial(crate::measure_decision_fixtures::Fixture::multiple());
    let group = request.capture();
    for omit in [false, true] {
        let mut fixture = DecisionReviewFixture::schedule(
            vec![reference_v2(&group.measures[0])],
            append_v2(&request.history, &group),
        );
        let entry = &mut fixture.decision_history.decisions[0];
        if omit {
            entry.capture.measures.pop();
        } else {
            entry.capture.measures[1]
                .result
                .projection
                .subject
                .display_name = "Invented sibling".into();
        }
        refresh_group(&mut entry.capture);
        entry.origin.group_digest = entry.capture.capture_digest;
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn c_after_m2_cannot_claim_an_older_judicial_owner_even_when_its_row_and_outer_digest_are_rehashed()
{
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let administrative = AdministrativeFixture::after(&group, &judicial.history, 0);
    let capture = administrative.capture();
    let original = append_administrative_decision_history(&administrative.history, &capture);
    let old = &original.records.judicial.groups[0].capture;
    for mutation in 0..3 {
        let mut forged = capture.clone();
        match mutation {
            0 => {
                forged.review.result.last_judicial = MeasureJudicialRef {
                    owner: owned(old).owner,
                    reference: reference(&old.measures[0]),
                }
            }
            1 => forged.review.result.values = group.measures[0].result.values.clone(),
            _ => {
                forged.review.result.sources.subject.changed_by.email =
                    "contradictory retained source".into()
            }
        }
        forged.records[0].result = forged.review.result.clone();
        refresh_administrative(&mut forged);
        let mut evidence = original.clone();
        let entry = evidence.records.administrative.last_mut().unwrap();
        entry.origin.submission_digest = forged.review.submission_digest;
        entry.origin.review_digest = forged.review.review_digest;
        entry.origin.capture_digest = forged.capture_digest;
        entry.capture = forged.clone();
        let fixture =
            DecisionReviewFixture::schedule(vec![record_reference(&forged.records[0])], evidence);
        assert!(fixture.prepare(None).is_err());
    }
}

#[test]
fn marked_c_after_m2_rejects_fresh_and_coherently_rehashed_historical_review() {
    let judicial = judicial_fixture();
    let group = judicial.capture();
    let original_history = append_v2(&judicial.history, &group);
    let valid =
        DecisionReviewFixture::schedule(vec![reference_v2(&group.measures[0])], original_history);
    let mut marking = AdministrativeFixture::after(&group, &judicial.history, 0);
    marking.command.action = MeasureAdministrativeAction::MarkEnteredInError;
    let marked = marking.capture();
    let evidence = append_administrative_decision_history(&marking.history, &marked);
    let invalid = DecisionReviewFixture::schedule(
        vec![record_reference(&marked.records[0])],
        evidence.clone(),
    );
    assert!(invalid.prepare(None).is_err());
    let original = valid.capture(None, marked.recorded_at);
    let replacement = DecisionReviewFixture::replace(
        &original,
        vec![record_reference(&marked.records[0])],
        evidence.clone(),
    );
    assert!(replacement.prepare(Some(&original)).is_err());
    let mut forged = original;
    retarget(&mut forged, record_reference(&marked.records[0]));
    assert!(precautionary_hearing_receipt_with_decision_history_matches(
        &Hasher, &forged, &evidence
    )
    .is_err());
    assert!(
        precautionary_hearing_origin_with_decision_history(&Hasher, &forged, &evidence).is_err()
    );
    let origin = claimed_hearing_origin(&forged);
    assert!(precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &[forged.clone()],
        &origin,
        &evidence
    )
    .is_err());
    assert!(DecisionReviewFixture::cancel(&forged, evidence)
        .prepare(Some(&forged))
        .is_err());
}
