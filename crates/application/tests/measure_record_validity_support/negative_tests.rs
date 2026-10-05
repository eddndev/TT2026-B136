use crate::validity_support::*;
use domain::crypto::{DocumentHasher, Sha256Digest};

fn refresh(value: &mut MeasureAdministrativeCapture) {
    let review = &mut value.review;
    review.submission_digest = Hasher.hash_bytes(
        &measure_administrative_submission_bytes(&review.actor, review.case_id, &review.command)
            .unwrap(),
    );
    review.review_digest = Hasher.hash_bytes(&measure_administrative_review_bytes(review).unwrap());
    for row in &mut value.records {
        row.review_digest = review.review_digest;
        row.capture_digest = Hasher.hash_bytes(&measure_administrative_record_bytes(row).unwrap());
    }
    value.capture_digest = Hasher.hash_bytes(&measure_administrative_capture_bytes(value).unwrap());
}

fn unverified_entry(value: MeasureAdministrativeCapture) -> MeasureAdministrativeEvidence {
    MeasureAdministrativeEvidence {
        origin: MeasureAdministrativeOrigin {
            case_id: value.review.case_id,
            operation_id: value.review.command.operation_id,
            submission_digest: value.review.submission_digest,
            review_digest: value.review.review_digest,
            capture_digest: value.capture_digest,
        },
        capture: value,
    }
}

fn rejected(value: &MeasureAdministrativeCapture, fixture: &RecordFixture) {
    assert!(
        measure_administrative_capture_with_history_matches(&Hasher, value, &fixture.history)
            .is_err()
    );
    assert!(measure_administrative_origin_with_history(&Hasher, value, &fixture.history).is_err());
    let mut history = fixture.history.clone();
    history.administrative.push(unverified_entry(value.clone()));
    let selected = record_reference(&value.records[0]);
    assert!(resolve_measure_records(&Hasher, fixture.case_id, &[selected], &history).is_err());
}

#[test]
fn entered_in_error_record_cannot_be_corrected_or_marked_again() {
    let fixture = marking(RecordFixture::initial());
    let prior = capture(&fixture);
    for mark_again in [false, true] {
        let mut next = RecordFixture::next(&prior, &fixture.history, 1);
        if mark_again {
            next = marking(next);
        }
        assert!(prepare(&next).is_err());
    }
}

#[test]
fn marking_retains_the_previous_administrative_declaration_even_after_coherent_rehashing() {
    let first = RecordFixture::initial();
    let correction = first.capture();
    let fixture = marking(RecordFixture::next(&correction, &first.history, 1));
    let original = capture(&fixture);
    for mutation in 0..11 {
        let mut changed = original.clone();
        let result = &mut changed.review.result;
        match mutation {
            0 => result.validity = MeasureCaptureValidity::Valid,
            1 => {
                result.values = first.history.judicial.groups[0].capture.measures[0]
                    .result
                    .values
                    .clone()
            }
            2 => {
                let mut input = crate::effect_support::values_input(&result.values);
                input.conditions = note("Uncommanded replacement conditions");
                result.values = MeasureValues::new(input);
            }
            3 => result.sources.subject.changed_by.email = "different source recorder".into(),
            4 => result.projection.subject.display_name = "Different retained subject".into(),
            5 => result.last_action = MeasureCaptureAction::Revoke,
            6 => {
                result.record_root = MeasureRecordRoot::Administrative {
                    operation_id: fixture.command.operation_id,
                    measure_id: result.id,
                }
            }
            7 => {
                result.previous = PrecautionaryMeasureRef::new(
                    result.previous.id(),
                    result.previous.revision(),
                    Sha256Digest::from_array([99; 32]),
                )
            }
            8 => result.revision = MeasureRevision::new(9).unwrap(),
            9 => result.last_judicial.reference = fixture.command.target,
            _ => result.id = id(999),
        }
        changed.records[0].result = changed.review.result.clone();
        refresh(&mut changed);
        rejected(&changed, &fixture);
    }
}

#[test]
fn mark_review_and_owned_row_must_agree_on_entered_in_error_validity() {
    let fixture = marking(RecordFixture::initial());
    let original = capture(&fixture);
    for change_review in [false, true] {
        let mut changed = original.clone();
        if change_review {
            changed.review.result.validity = MeasureCaptureValidity::Valid;
        } else {
            changed.records[0].result.validity = MeasureCaptureValidity::Valid;
        }
        refresh(&mut changed);
        rejected(&changed, &fixture);
    }
}

#[test]
fn a_rehashed_mark_cannot_be_relabelled_as_an_unchanged_correction() {
    let fixture = marking(RecordFixture::initial());
    let mut changed = capture(&fixture);
    let values = &changed.review.result.values;
    let supervision = match values.supervision() {
        MeasureSupervision::Known { statement, .. } => statement,
        MeasureSupervision::Unknown { reason } => reason,
    };
    changed.review.command.action =
        MeasureAdministrativeAction::Correct(MeasureCorrectionValues::new(
            values.conditions().clone(),
            values.validity().clone(),
            supervision.clone(),
        ));
    changed.review.result.validity = MeasureCaptureValidity::Valid;
    changed.records[0].result = changed.review.result.clone();
    refresh(&mut changed);
    rejected(&changed, &fixture);
}

#[test]
fn rewriting_a_marked_ancestor_as_valid_does_not_authorize_a_later_correction() {
    let fixture = marking(RecordFixture::initial());
    let marked = capture(&fixture);
    let mut next = RecordFixture::next(&marked, &fixture.history, 1);
    let mut forged = marked;
    forged.review.result.validity = MeasureCaptureValidity::Valid;
    forged.records[0].result.validity = MeasureCaptureValidity::Valid;
    refresh(&mut forged);
    next.command.target = record_reference(&forged.records[0]);
    next.history.administrative[0] = unverified_entry(forged);
    assert!(prepare(&next).is_err());
}

#[test]
fn mark_only_receipt_cannot_drop_duplicate_or_add_a_replacement_record() {
    let fixture = marking(RecordFixture::initial());
    let original = capture(&fixture);
    for mutation in 0..3 {
        let mut changed = original.clone();
        let selected = record_reference(&changed.records[0]);
        match mutation {
            0 => changed.records.clear(),
            1 => changed.records.push(changed.records[0].clone()),
            _ => {
                let mut extra = changed.records[0].clone();
                extra.result.id = id(999);
                extra.result.validity = MeasureCaptureValidity::Valid;
                extra.capture_digest =
                    Hasher.hash_bytes(&measure_administrative_record_bytes(&extra).unwrap());
                changed.records.push(extra);
            }
        }
        if let Ok(bytes) = measure_administrative_capture_bytes(&changed) {
            changed.capture_digest = Hasher.hash_bytes(&bytes);
        }
        assert!(measure_administrative_capture_with_history_matches(
            &Hasher,
            &changed,
            &fixture.history
        )
        .is_err());
        let mut history = fixture.history.clone();
        history.administrative.push(unverified_entry(changed));
        assert!(resolve_measure_records(&Hasher, fixture.case_id, &[selected], &history).is_err());
    }
}
