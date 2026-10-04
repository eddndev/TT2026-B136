use super::*;
use domain::crypto::Sha256Digest;
use time::Duration;
use uuid::Uuid;

#[test]
fn review_requires_real_measure_history_at_every_public_boundary() {
    let group = MeasureFixture::single().capture();
    let fixture = ReviewFixture::schedule(&group);
    assert!(fixture.hearing.clone().prepare(None).is_err());
    let mut missing = fixture.clone();
    missing.measure_history = empty_history();
    assert!(missing.prepare(None).is_err());
    let evidence = fixture.measure_history.clone();
    let capture = fixture.capture(None, group.recorded_at);
    let origin =
        precautionary_hearing_origin_with_measure_history(&Hasher, &capture, &evidence).unwrap();
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher,
        &capture,
        &empty_history()
    )
    .is_err());
    assert!(
        precautionary_hearing_origin_with_measure_history(&Hasher, &capture, &empty_history())
            .is_err()
    );
    assert!(precautionary_hearing_history_with_measure_history_matches(
        &Hasher,
        &[capture],
        &origin,
        &empty_history()
    )
    .is_err());
}

#[test]
fn review_targets_require_exact_measure_identity_revision_and_capture_digest() {
    let group = MeasureFixture::single().capture();
    let original = &group.measures[0];
    for mutation in 0..3 {
        let mut fixture = ReviewFixture::schedule(&group);
        let selected = PrecautionaryMeasureRef::new(
            if mutation == 0 {
                crate::measure_decision_fixtures::id(80)
            } else {
                original.result.id
            },
            if mutation == 1 {
                MeasureRevision::new(2).unwrap()
            } else {
                original.result.revision
            },
            if mutation == 2 {
                Sha256Digest::from_array([99; 32])
            } else {
                original.capture_digest
            },
        );
        select_targets(&mut fixture.hearing, vec![selected]);
        assert!(fixture.prepare(None).is_err());
    }
}

fn foreign_group() -> MeasureDecisionGroupCapture {
    let mut fixture = MeasureFixture::single();
    let case = domain::cases::CaseId::from_uuid(Uuid::from_u128(2));
    fixture.case_id = case;
    let mut context = fixture.material.context.material().clone();
    context.case_id = case;
    context.administration.case_id = case;
    context.stage_administration.case_id = case;
    let application::case_stages::CaseStageEntry::Initial(stage) = &mut context.stage else {
        unreachable!()
    };
    stage.case_id = case;
    fixture.material.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    fixture.command.context =
        crate::measure_decision_fixtures::expectation(&fixture.material.context);
    let sources = &mut fixture.material.result_sources[0].sources;
    sources.subject.case_id = case;
    let supervisor = sources.supervisor.as_mut().unwrap();
    crate::participant_support::typed_mut(supervisor).case_id = case;
    supervisor.bound_subject.as_mut().unwrap().case_id = case;
    fixture.capture()
}

#[test]
fn a_valid_foreign_case_group_cannot_supply_a_review_target() {
    let group = foreign_group();
    assert!(ReviewFixture::schedule(&group).prepare(None).is_err());
}

#[test]
fn selecting_one_measure_still_requires_the_integrity_of_its_whole_owning_group() {
    let group = MeasureFixture::multiple().capture();
    let mut fixture = ReviewFixture::schedule(&group);
    select_targets(
        &mut fixture.hearing,
        vec![crate::measure_decision_fixtures::reference(
            &group.measures[0],
        )],
    );
    let evidence = &mut fixture.measure_history.groups[0];
    evidence.capture.measures[1]
        .result
        .projection
        .subject
        .display_name = "Forged sibling label".into();
    crate::measure_decision_fixtures::refresh_digests(&mut evidence.capture);
    evidence.origin.group_digest = evidence.capture.capture_digest;
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn a_later_target_requires_every_owning_group_dependency() {
    let first = MeasureFixture::single().capture();
    let later = MeasureChangeFixture::confirm(&first).capture();
    let mut fixture = ReviewFixture::schedule(&first);
    select_targets(&mut fixture.hearing, targets(&later));
    fixture.measure_history =
        crate::measure_decision_effect_support::append_history(&history(&first), &later);
    assert!(fixture.clone().prepare(None).is_ok());
    fixture.measure_history.groups.remove(0);
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn review_and_imposition_wrappers_reject_unused_measure_history() {
    let first = MeasureFixture::single().capture();
    let later = MeasureChangeFixture::confirm(&first).capture();
    let mut review = ReviewFixture::schedule(&first);
    review.measure_history =
        crate::measure_decision_effect_support::append_history(&history(&first), &later);
    assert!(review.prepare(None).is_err());
    let evidence = history(&first);
    let imposition = ReviewFixture {
        hearing: HearingFixture::schedule(),
        measure_history: evidence.clone(),
    };
    assert!(imposition.prepare(None).is_err());
    let capture = HearingFixture::schedule().capture(None, first.recorded_at);
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher, &capture, &evidence
    )
    .is_err());
}

#[test]
fn checked_review_capture_cannot_predate_the_selected_measure_capture() {
    let group = MeasureFixture::single().capture();
    let fixture = ReviewFixture::schedule(&group);
    assert!(fixture
        .clone()
        .prepare(None)
        .unwrap()
        .into_capture(&Hasher, group.recorded_at - Duration::nanoseconds(1))
        .is_err());
    fixture
        .prepare(None)
        .unwrap()
        .into_capture(&Hasher, group.recorded_at)
        .unwrap();
}

#[test]
fn rehashed_receipt_cannot_predate_its_selected_measure_capture() {
    let group = MeasureFixture::single().capture();
    let fixture = ReviewFixture::schedule(&group);
    let evidence = fixture.measure_history.clone();
    let mut capture = fixture.capture(None, group.recorded_at);
    capture.recorded_at -= Duration::nanoseconds(1);
    refresh(&mut capture);
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher, &capture, &evidence
    )
    .is_err());
}

#[test]
fn replacement_and_cancellation_reject_a_rehashed_predecessor_before_its_measure_sources() {
    let group = MeasureFixture::single().capture();
    let fixture = ReviewFixture::schedule(&group);
    let evidence = fixture.measure_history.clone();
    let mut predecessor = fixture.capture(None, group.recorded_at);
    predecessor.recorded_at -= Duration::nanoseconds(1);
    refresh(&mut predecessor);
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher,
        &predecessor,
        &evidence
    )
    .is_err());
    let replacement = ReviewFixture::replace(&predecessor, targets(&group), evidence.clone());
    assert!(replacement.prepare(Some(&predecessor)).is_err());
    let cancellation = ReviewFixture::cancel(&predecessor, evidence);
    assert!(cancellation.prepare(Some(&predecessor)).is_err());
}

#[test]
fn rehashed_target_references_cannot_replace_real_capture_evidence() {
    let group = MeasureFixture::single().capture();
    let fixture = ReviewFixture::schedule(&group);
    let evidence = fixture.measure_history.clone();
    let mut capture = fixture.capture(None, group.recorded_at);
    let mut values = values_input(&capture.review.resolved_values);
    values.review_targets = vec![PrecautionaryMeasureRef::new(
        group.measures[0].result.id,
        MeasureRevision::new(2).unwrap(),
        Sha256Digest::from_array([99; 32]),
    )];
    capture.review.resolved_values = PrecautionaryHearingValues::new(values).unwrap();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut capture.review.command.change
    else {
        unreachable!()
    };
    *values = capture.review.resolved_values.clone();
    refresh(&mut capture);
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher, &capture, &evidence
    )
    .is_err());
}

#[test]
fn historical_review_targets_do_not_assert_the_latest_measure_head_or_current_legal_effect() {
    let first = MeasureFixture::single().capture();
    let mut revoked = MeasureChangeFixture::confirm(&first);
    revoked.effects(vec![
        domain::precautionary_measures::MeasureEffect::Revoke {
            previous: crate::measure_decision_fixtures::reference(&first.measures[0]),
        },
    ]);
    let later = revoked.capture();
    let old = ReviewFixture::schedule(&first).capture(None, later.recorded_at);
    precautionary_hearing_receipt_with_measure_history_matches(&Hasher, &old, &history(&first))
        .unwrap();
    let mut current = ReviewFixture::schedule(&first);
    select_targets(&mut current.hearing, targets(&later));
    current.measure_history =
        crate::measure_decision_effect_support::append_history(&history(&first), &later);
    let evidence = current.measure_history.clone();
    let capture = current.capture(None, later.recorded_at);
    precautionary_hearing_receipt_with_measure_history_matches(&Hasher, &capture, &evidence)
        .unwrap();
}
