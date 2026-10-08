use super::*;
use application::cases::CaseRevision;
use domain::{
    hearings::HearingSupportRef, precautionary_hearings::PrecautionaryHearingSchedulingBasis,
};

fn advanced() -> (RecordFixture, MeasureAdministrativeCapture) {
    let mut fixture = RecordFixture::initial();
    let mut context = fixture.context.material().clone();
    context.administration.revision = CaseRevision::new(2).unwrap();
    context.administration.changed_at = fixture.recorded_at - Duration::seconds(1);
    fixture.context = PrecautionaryContext::new(&Hasher, context).unwrap();
    fixture.command.context = crate::measure_decision_fixtures::expectation(&fixture.context);
    let capture = fixture.capture();
    (fixture, capture)
}

#[test]
fn review_uses_selected_administrative_context_instead_of_older_judicial_context() {
    let (fixture, prior) = advanced();
    let (original, history) = review_fixture(&prior, &fixture.history);
    let valid = prepare(&original, &history, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    for mutation in 0..3 {
        let mut hearing = original.clone();
        let mut material = if mutation == 0 {
            crate::context_support::initial()
        } else {
            hearing.context.material().clone()
        };
        match mutation {
            1 => {
                material.administration.revision = CaseRevision::new(3).unwrap();
                material.administration.changed_at -= Duration::seconds(1);
            }
            2 => {
                material.administration.changed_by.email =
                    "different historical administrator".into()
            }
            _ => {}
        }
        let context = PrecautionaryContext::new(&Hasher, material).unwrap();
        set_context(&mut hearing, context.clone());
        assert!(prepare(&hearing, &history, None).is_err());
        let mut forged = valid.clone();
        forged.review.command = hearing.command;
        forged.review.scheduling_context = context.clone();
        forged.review.observed_context = context;
        refresh(&mut forged);
        assert!(precautionary_hearing_receipt_with_record_history_matches(
            &Hasher, &forged, &history
        )
        .is_err());
    }
}

#[test]
fn hearing_participant_subject_metadata_must_agree_with_its_administrative_target_sources() {
    let (fixture, prior) = corrected();
    let (mut hearing, history) = review_fixture(&prior, &fixture.history);
    let bound = hearing.sources.participants[1]
        .bound_subject
        .as_mut()
        .unwrap();
    let retained = prior
        .review
        .result
        .sources
        .supervisor
        .as_ref()
        .unwrap()
        .bound_subject
        .as_ref()
        .unwrap();
    assert_eq!(bound.id, retained.id);
    assert_eq!(bound.revision, retained.revision);
    bound.changed_by.email = "contradictory historical subject author".into();
    assert!(prepare(&hearing, &history, None).is_err());
}

#[test]
fn scheduling_document_metadata_must_agree_with_retained_judicial_support() {
    let (fixture, prior) = corrected();
    let (mut hearing, history) = review_fixture(&prior, &fixture.history);
    hearing.sources.support = prior.review.support.clone();
    hearing.sources.support.name = "contradictory-judicial-support.pdf".into();
    let mut input = crate::precautionary_receipt_support::values_input();
    input.purpose = PrecautionaryHearingPurpose::Review;
    input.review_targets = vec![record_reference(&prior.records[0])];
    input.scheduling_basis = PrecautionaryHearingSchedulingBasis::new(
        note("Communicated appointment"),
        HearingSupportRef::new(
            hearing.sources.support.reference,
            hearing.sources.support.digest,
        ),
        note("Page 1"),
    );
    set_values(
        &mut hearing,
        PrecautionaryHearingValues::new(input).unwrap(),
    );
    assert!(prepare(&hearing, &history, None).is_err());
}

#[test]
fn rehashed_hearing_cannot_rewrite_the_exact_administrative_reference() {
    let (fixture, prior) = corrected();
    let (hearing, history) = review_fixture(&prior, &fixture.history);
    let original = prepare(&hearing, &history, None)
        .unwrap()
        .into_capture(&Hasher, prior.recorded_at)
        .unwrap();
    let selected = record_reference(&prior.records[0]);
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
            domain::crypto::Sha256Digest::from_array([99; 32]),
        ),
    ] {
        let mut changed_hearing = hearing.clone();
        select(&mut changed_hearing, target);
        assert!(prepare(&changed_hearing, &history, None).is_err());
        let PrecautionaryHearingChange::Schedule { values, .. } = &changed_hearing.command.change
        else {
            unreachable!()
        };
        let mut forged = original.clone();
        forged.review.resolved_values = values.clone();
        forged.review.command = changed_hearing.command;
        refresh(&mut forged);
        assert!(precautionary_hearing_receipt_with_record_history_matches(
            &Hasher, &forged, &history
        )
        .is_err());
    }
}
