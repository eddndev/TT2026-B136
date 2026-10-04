use super::*;
use application::cases::CaseRevision;
use domain::hearings::{HearingParticipantRef, HearingSupportRef, HearingTime};
use domain::precautionary_measures::MeasureDecisionValues;
use time::{Duration, OffsetDateTime};

fn advanced(context: &PrecautionaryContext, revision: u32, seconds: i64) -> PrecautionaryContext {
    let mut material = context.material().clone();
    material.administration.revision = CaseRevision::new(revision).unwrap();
    material.administration.changed_at += Duration::seconds(seconds);
    PrecautionaryContext::new(&Hasher, material).unwrap()
}

fn group_with_context(context: PrecautionaryContext) -> MeasureDecisionGroupCapture {
    let mut fixture = MeasureFixture::single();
    fixture.material.context = context;
    fixture.command.context =
        crate::measure_decision_fixtures::expectation(&fixture.material.context);
    fixture.capture()
}

fn imposition_control(hearing: &HearingFixture) -> HearingFixture {
    let mut result = hearing.clone();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut result.command.change else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.purpose = PrecautionaryHearingPurpose::Imposition;
    input.review_targets.clear();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    result
}

#[test]
fn review_scheduling_context_must_advance_from_the_selected_group_context() {
    let initial = crate::precautionary_receipt_support::context();
    let context = advanced(&initial, 2, 20);
    let group = group_with_context(context.clone());
    assert!(ReviewFixture::schedule(&group).prepare(None).is_err());
    let mut consistent = ReviewFixture::schedule(&group);
    set_context(&mut consistent.hearing, context);
    assert!(consistent.prepare(None).is_ok());
}

#[test]
fn higher_review_administration_revision_cannot_hide_a_backward_context_clock() {
    let context = advanced(&crate::precautionary_receipt_support::context(), 2, 20);
    let group = group_with_context(context.clone());
    let mut fixture = ReviewFixture::schedule(&group);
    set_context(&mut fixture.hearing, advanced(&context, 3, -1));
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn higher_review_stage_revision_cannot_hide_a_backward_stage_capture_clock() {
    let mut earlier = crate::context_support::changed(crate::context_support::intermediate());
    crate::context_support::changed_mut(&mut earlier).recorded_at =
        crate::context_support::at() + Duration::seconds(20);
    let group = group_with_context(PrecautionaryContext::new(&Hasher, earlier).unwrap());
    let mut later = crate::context_support::changed(crate::context_support::trial());
    crate::context_support::changed_mut(&mut later).recorded_at =
        crate::context_support::at() + Duration::seconds(19);
    let mut fixture = ReviewFixture::schedule(&group);
    set_context(
        &mut fixture.hearing,
        PrecautionaryContext::new(&Hasher, later).unwrap(),
    );
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn cancellation_observed_context_cannot_mask_an_older_retained_scheduling_context() {
    let original = crate::precautionary_receipt_support::context();
    let selected_context = advanced(&original, 2, 20);
    let group = group_with_context(selected_context.clone());
    let mut fixture = ReviewFixture::schedule(&group);
    set_context(&mut fixture.hearing, selected_context.clone());
    let evidence = fixture.measure_history.clone();
    let first = fixture.capture(None, group.recorded_at);
    let mut cancellation = ReviewFixture::cancel(&first, evidence.clone());
    set_context(&mut cancellation.hearing, advanced(&selected_context, 3, 1));
    let mut cancelled =
        cancellation.capture(Some(&first), first.recorded_at + Duration::seconds(2));
    precautionary_hearing_receipt_with_measure_history_matches(&Hasher, &cancelled, &evidence)
        .unwrap();
    cancelled.review.scheduling_context = original;
    refresh(&mut cancelled);
    assert!(precautionary_hearing_receipt_with_measure_history_matches(
        &Hasher, &cancelled, &evidence
    )
    .is_err());
}

#[test]
fn equal_context_revisions_require_the_same_full_captured_provenance() {
    let group = MeasureFixture::single().capture();
    let mut fixture = ReviewFixture::schedule(&group);
    let mut context = fixture.hearing.context.material().clone();
    context.administration.changed_by.email = "different@example.test".into();
    context.stage_administration = context.administration.clone();
    let application::case_stages::CaseStageEntry::Initial(stage) = &mut context.stage else {
        unreachable!()
    };
    stage.recorded_by = context.administration.changed_by.clone();
    set_context(
        &mut fixture.hearing,
        PrecautionaryContext::new(&Hasher, context).unwrap(),
    );
    assert!(imposition_control(&fixture.hearing).prepare(None).is_ok());
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn a_bound_subject_copied_into_the_hearing_must_match_its_measure_group_source() {
    let group = MeasureFixture::single().capture();
    let mut fixture = ReviewFixture::schedule(&group);
    fixture.hearing.sources.participants[1]
        .bound_subject
        .as_mut()
        .unwrap()
        .changed_by
        .email = "different@example.test".into();
    assert!(imposition_control(&fixture.hearing).prepare(None).is_ok());
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn an_exact_participant_copied_into_the_hearing_must_preserve_its_full_provenance() {
    let group = MeasureFixture::single().capture();
    let mut fixture = ReviewFixture::schedule(&group);
    fixture.hearing.sources.participants[1] =
        group.measures[0].result.sources.supervisor.clone().unwrap();
    crate::participant_support::typed_mut(&mut fixture.hearing.sources.participants[1])
        .changed_by
        .email = "different@example.test".into();
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.hearing.command.change
    else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.participants = fixture
        .hearing
        .sources
        .participants
        .iter()
        .map(|detail| HearingParticipantRef::new(detail.id(), detail.revision_number()))
        .collect();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    assert!(imposition_control(&fixture.hearing).prepare(None).is_ok());
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn a_support_shared_by_the_hearing_and_measure_group_requires_identical_metadata() {
    let mut measure = MeasureFixture::single();
    measure.material.support = HearingFixture::schedule().sources.support;
    let mut values = crate::measure_decision_fixtures::decision_input(&measure.command.values);
    values.support = HearingSupportRef::new(
        measure.material.support.reference,
        measure.material.support.digest,
    );
    measure.command.values = MeasureDecisionValues::new(values);
    let group = measure.capture();
    assert!(ReviewFixture::schedule(&group).prepare(None).is_ok());
    let mut fixture = ReviewFixture::schedule(&group);
    fixture.hearing.sources.support.name = "different-name.pdf".into();
    assert!(imposition_control(&fixture.hearing).prepare(None).is_ok());
    assert!(fixture.prepare(None).is_err());
}

#[test]
fn scheduled_time_remains_a_declaration_independent_of_the_selected_measure_capture_clock() {
    let group = MeasureFixture::single().capture();
    let mut fixture = ReviewFixture::schedule(&group);
    let PrecautionaryHearingChange::Schedule { values, .. } = &mut fixture.hearing.command.change
    else {
        unreachable!()
    };
    let mut input = values_input(values);
    input.scheduled_at = HearingTime::new(OffsetDateTime::UNIX_EPOCH).unwrap();
    *values = PrecautionaryHearingValues::new(input).unwrap();
    let evidence = fixture.measure_history.clone();
    let capture = fixture.capture(None, group.recorded_at);
    assert_eq!(
        capture.review.resolved_values.scheduled_at().value(),
        OffsetDateTime::UNIX_EPOCH
    );
    precautionary_hearing_receipt_with_measure_history_matches(&Hasher, &capture, &evidence)
        .unwrap();
}
