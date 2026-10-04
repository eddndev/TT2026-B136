use super::*;
use domain::crypto::DocumentHasher;

fn refresh(capture: &mut MeasureAdministrativeCapture) {
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

fn rejected(capture: &MeasureAdministrativeCapture, history: &MeasureHistoryEvidence) {
    assert!(measure_administrative_capture_matches(&Hasher, capture, history).is_err());
    assert!(measure_administrative_origin(&Hasher, capture, history).is_err());
}

#[test]
fn coherent_rehashed_result_fields_are_rederived_from_the_exact_judicial_target() {
    let fixture = CorrectionFixture::initial();
    let original = fixture.capture();
    for mutation in 0..18 {
        let mut changed = original.clone();
        let result = &mut changed.review.result;
        let previous = result.previous;
        match mutation {
            0 => result.id = id(999),
            1 => result.revision = MeasureRevision::new(5).unwrap(),
            2 => {
                result.previous = PrecautionaryMeasureRef::new(
                    previous.id(),
                    previous.revision(),
                    Sha256Digest::from_array([99; 32]),
                )
            }
            3 => {
                result.record_root = MeasureRecordRoot::Administrative {
                    operation_id: fixture.command.operation_id,
                    measure_id: result.id,
                }
            }
            4 => {
                result.judicial_origin.decision_id =
                    MeasureDecisionId::from_uuid(Uuid::from_u128(999))
            }
            5 => {
                result.last_judicial.owner.operation_id =
                    MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999))
            }
            6 => {
                result.last_judicial.owner.decision_id =
                    MeasureDecisionId::from_uuid(Uuid::from_u128(999))
            }
            7 => result.last_judicial.owner.group_digest = Sha256Digest::from_array([99; 32]),
            8 => {
                result.last_judicial.reference =
                    PrecautionaryMeasureRef::new(id(999), previous.revision(), previous.digest())
            }
            9 => result.last_action = MeasureCaptureAction::Revoke,
            10 => result.validity = MeasureCaptureValidity::EnteredInError,
            11 => {
                let mut values = crate::effect_support::values_input(&result.values);
                values.conditions = note("Uncommanded corrected conditions");
                result.values = MeasureValues::new(values);
            }
            12 => {
                result.sources.subject.changed_by.email =
                    "different retained subject recorder".into()
            }
            13 => {
                crate::participant_support::typed_mut(result.sources.supervisor.as_mut().unwrap())
                    .changed_by
                    .email = "different retained supervisor recorder".into()
            }
            14 => result.projection.subject.display_name = "Invented subject projection".into(),
            15 => {
                result
                    .projection
                    .supervisor
                    .as_mut()
                    .unwrap()
                    .overview
                    .display_name = "Invented supervisor projection".into()
            }
            16 => {
                result.record_root = MeasureRecordRoot::Judicial(MeasureOriginIds {
                    decision_id: result.judicial_origin.decision_id,
                    operation_id: MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999)),
                })
            }
            _ => {
                result.last_judicial.reference = PrecautionaryMeasureRef::new(
                    previous.id(),
                    MeasureRevision::new(2).unwrap(),
                    previous.digest(),
                )
            }
        }
        changed.records[0].result = changed.review.result.clone();
        refresh(&mut changed);
        rejected(&changed, &fixture.history);
    }
}

#[test]
fn each_record_must_bind_the_exact_review_operation_provenance_context_and_support() {
    let fixture = CorrectionFixture::initial();
    let original = fixture.capture();
    for mutation in 0..8 {
        let mut changed = original.clone();
        let record = &mut changed.records[0];
        match mutation {
            0 => record.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
            1 => {
                record.operation_id = MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(999))
            }
            2 => record.actor.email = "different administrative recorder".into(),
            3 => record.actor.role = domain::identity::Role::Owner,
            4 => {
                let mut context = record.context.material().clone();
                context.administration.revision = CaseRevision::new(2).unwrap();
                context.administration.changed_at += Duration::seconds(1);
                record.context = PrecautionaryContext::new(&Hasher, context).unwrap();
            }
            5 => record.support.name = "different-retained-support.pdf".into(),
            6 => record.recorded_at += Duration::seconds(1),
            _ => record.result.values = fixture.previous().result.values,
        }
        refresh(&mut changed);
        rejected(&changed, &fixture.history);
    }
}

#[test]
fn record_review_digest_and_outer_receipt_digest_are_independently_checked() {
    let fixture = CorrectionFixture::initial();
    let original = fixture.capture();
    for mutation in 0..4 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.review.submission_digest = Sha256Digest::from_array([99; 32]),
            1 => changed.review.review_digest = Sha256Digest::from_array([99; 32]),
            2 => changed.records[0].review_digest = Sha256Digest::from_array([99; 32]),
            _ => changed.records[0].capture_digest = Sha256Digest::from_array([99; 32]),
        }
        changed.capture_digest =
            Hasher.hash_bytes(&measure_administrative_capture_bytes(&changed).unwrap());
        rejected(&changed, &fixture.history);
    }
}

#[test]
fn first_correction_receipt_owns_exactly_one_complete_record() {
    let fixture = CorrectionFixture::initial();
    let original = fixture.capture();
    for mutation in 0..3 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.records.clear(),
            1 => changed.records.push(changed.records[0].clone()),
            _ => {
                let mut extra = changed.records[0].clone();
                extra.result.id = id(999);
                extra.capture_digest =
                    Hasher.hash_bytes(&measure_administrative_record_bytes(&extra).unwrap());
                changed.records.push(extra);
            }
        }
        if let Ok(bytes) = measure_administrative_capture_bytes(&changed) {
            changed.capture_digest = Hasher.hash_bytes(&bytes);
        }
        rejected(&changed, &fixture.history);
    }
}

#[test]
fn correction_support_cannot_revert_to_the_initial_imposition_after_a_later_decision() {
    let fixture = later_with_different_support();
    let mut changed = fixture.capture();
    let initial_support = fixture.history.groups[0].capture.decision.support.clone();
    assert_ne!(initial_support, changed.review.support);
    changed.review.support = initial_support.clone();
    changed.records[0].support = initial_support;
    refresh(&mut changed);
    rejected(&changed, &fixture.history);
}

#[test]
fn receipt_and_origin_validation_require_the_complete_original_target_closure() {
    let fixture = later_with_different_support();
    let capture = fixture.capture();
    for omit in 0..fixture.history.groups.len() {
        let mut history = fixture.history.clone();
        history.groups.remove(omit);
        rejected(&capture, &history);
    }
}
