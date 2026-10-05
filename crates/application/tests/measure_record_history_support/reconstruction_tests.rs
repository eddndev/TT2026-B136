use super::*;
use domain::crypto::DocumentHasher;

pub(super) fn refresh(capture: &mut MeasureAdministrativeCapture) {
    let review = &mut capture.review;
    review.submission_digest = Hasher.hash_bytes(
        &measure_administrative_submission_bytes(&review.actor, review.case_id, &review.command)
            .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&measure_administrative_review_bytes(review).unwrap());
    for record in &mut capture.records {
        record.review_digest = review.review_digest;
        record.capture_digest =
            Hasher.hash_bytes(&measure_administrative_record_bytes(record).unwrap());
    }
    capture.capture_digest =
        Hasher.hash_bytes(&measure_administrative_capture_bytes(capture).unwrap());
}

fn unverified_entry(capture: MeasureAdministrativeCapture) -> MeasureAdministrativeEvidence {
    MeasureAdministrativeEvidence {
        origin: MeasureAdministrativeOrigin {
            case_id: capture.review.case_id,
            operation_id: capture.review.command.operation_id,
            submission_digest: capture.review.submission_digest,
            review_digest: capture.review.review_digest,
            capture_digest: capture.capture_digest,
        },
        capture,
    }
}

pub(super) fn reject(capture: &MeasureAdministrativeCapture, fixture: &RecordFixture) {
    assert!(measure_administrative_capture_with_history_matches(
        &Hasher,
        capture,
        &fixture.history
    )
    .is_err());
    assert!(
        measure_administrative_origin_with_history(&Hasher, capture, &fixture.history).is_err()
    );
    let mut evidence = fixture.history.clone();
    evidence
        .administrative
        .push(unverified_entry(capture.clone()));
    let selected = record_reference(&capture.records[0]);
    assert!(resolve_measure_records(&Hasher, fixture.case_id, &[selected], &evidence).is_err());
}

#[test]
fn rehashed_repeated_result_must_be_derived_from_the_selected_record_and_actual_judicial_source() {
    let (fixture, original, _) = chain();
    for mutation in 0..10 {
        let mut capture = original.clone();
        let result = &mut capture.review.result;
        match mutation {
            0 => result.revision = MeasureRevision::new(9).unwrap(),
            1 => result.previous = result.last_judicial.reference,
            2 => result.last_judicial.reference = fixture.command.target,
            3 => result.last_judicial.owner.group_digest = Sha256Digest::from_array([99; 32]),
            4 => {
                result.record_root = MeasureRecordRoot::Administrative {
                    operation_id: fixture.history.administrative[0].origin.operation_id,
                    measure_id: result.id,
                }
            }
            5 => result.validity = MeasureCaptureValidity::EnteredInError,
            6 => result.last_action = MeasureCaptureAction::Revoke,
            7 => result.sources.subject.changed_by.email = "rewritten historical subject".into(),
            8 => result.projection.subject.display_name = "Rewritten historical display".into(),
            _ => {
                result.values = fixture.history.administrative[0]
                    .capture
                    .review
                    .result
                    .values
                    .clone()
            }
        }
        capture.records[0].result = capture.review.result.clone();
        refresh(&mut capture);
        reject(&capture, &fixture);
    }
}

#[test]
fn a_repeated_owner_still_requires_exactly_its_complete_reconstructed_record_set() {
    let (fixture, original, _) = chain();
    for mutation in 0..3 {
        let mut capture = original.clone();
        let selected = record_reference(&original.records[0]);
        match mutation {
            0 => capture.records.clear(),
            1 => capture.records.push(capture.records[0].clone()),
            _ => {
                let mut extra = capture.records[0].clone();
                extra.result.id = id(999);
                extra.capture_digest =
                    Hasher.hash_bytes(&measure_administrative_record_bytes(&extra).unwrap());
                capture.records.push(extra);
            }
        }
        if let Ok(bytes) = measure_administrative_capture_bytes(&capture) {
            capture.capture_digest = Hasher.hash_bytes(&bytes);
        }
        assert!(measure_administrative_capture_with_history_matches(
            &Hasher,
            &capture,
            &fixture.history
        )
        .is_err());
        let mut evidence = fixture.history.clone();
        evidence.administrative.push(unverified_entry(capture));
        assert!(resolve_measure_records(&Hasher, fixture.case_id, &[selected], &evidence).is_err());
    }
}

#[test]
fn administrative_dependency_cycles_and_self_references_reject() {
    let (fixture, capture, history) = chain();
    for self_reference in [false, true] {
        let mut evidence = history.clone();
        let target = if self_reference {
            record_reference(&evidence.administrative[0].capture.records[0])
        } else {
            record_reference(&capture.records[0])
        };
        let first = &mut evidence.administrative[0].capture;
        first.review.command.target = target;
        first.review.result.previous = target;
        first.records[0].result.previous = target;
        refresh(first);
        let changed_first = unverified_entry(first.clone());
        evidence.administrative[0] = changed_first;
        let prior = record_reference(&evidence.administrative[0].capture.records[0]);
        let second = &mut evidence.administrative[1].capture;
        second.review.command.target = prior;
        second.review.result.previous = prior;
        second.records[0].result.previous = prior;
        refresh(second);
        let changed_second = unverified_entry(second.clone());
        evidence.administrative[1] = changed_second;
        let selected = record_reference(&evidence.administrative[1].capture.records[0]);
        assert!(resolve_measure_records(&Hasher, fixture.case_id, &[selected], &evidence).is_err());
    }
}

#[test]
fn a_forged_nonconsecutive_ancestor_cannot_be_hidden_behind_a_rehashed_child() {
    let (fixture, original, _) = chain();
    let mut ancestors = fixture.history.clone();
    let first = &mut ancestors.administrative[0].capture;
    first.review.result.revision = MeasureRevision::new(8).unwrap();
    first.records[0].result = first.review.result.clone();
    refresh(first);
    let previous = record_reference(&first.records[0]);
    let changed_first = unverified_entry(first.clone());
    ancestors.administrative[0] = changed_first;
    let mut capture = original;
    capture.review.command.target = previous;
    capture.review.result.previous = previous;
    capture.review.result.revision = MeasureRevision::new(9).unwrap();
    capture.records[0].result = capture.review.result.clone();
    refresh(&mut capture);
    let mut changed = fixture;
    changed.history = ancestors;
    reject(&capture, &changed);
}

#[test]
fn repeated_correction_retains_the_last_actual_decision_support_after_all_administrative_changes() {
    use domain::{crypto::DocumentId, hearings::HearingSupportRef};
    let initial = crate::measure_decision_fixtures::Fixture::single().capture();
    let mut later = crate::effect_support::LaterFixture::confirm(&initial);
    later.request.material.support.reference.id = DocumentId::from_uuid(Uuid::from_u128(92));
    later.request.material.support.digest = Sha256Digest::from_array([12; 32]);
    later.request.material.support.name = "later-decision.pdf".into();
    let mut values =
        crate::measure_decision_fixtures::decision_input(&later.request.command.values);
    values.support = HearingSupportRef::new(
        later.request.material.support.reference,
        later.request.material.support.digest,
    );
    later.request.command.values = MeasureDecisionValues::new(values);
    let ancestors = later.evidence.clone();
    let group = later.capture();
    let first =
        RecordFixture::from_first(CorrectionFixture::from_group(&group, &ancestors, id(70)));
    let initial_correction = first.capture();
    let second = RecordFixture::next(&initial_correction, &first.history, 1);
    let mut capture = second.capture();
    assert_eq!(capture.review.support, group.decision.support);
    assert_ne!(capture.review.support, initial.decision.support);
    capture.review.support = initial.decision.support.clone();
    capture.records[0].support = initial.decision.support;
    refresh(&mut capture);
    reject(&capture, &second);
}
