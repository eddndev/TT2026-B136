use super::*;

fn refresh(operation: &mut MeasureAdministrativeStoredOperation) {
    let capture = &mut operation.capture;
    capture.review.review_digest =
        Hasher.hash_bytes(&measure_administrative_review_bytes(&capture.review).unwrap());
    for row in &mut capture.records {
        row.review_digest = capture.review.review_digest;
        row.capture_digest = Hasher.hash_bytes(&measure_administrative_record_bytes(row).unwrap());
    }
    capture.capture_digest =
        Hasher.hash_bytes(&measure_administrative_capture_bytes(capture).unwrap());
    operation.origin.review_digest = capture.review.review_digest;
    operation.origin.capture_digest = capture.capture_digest;
}

#[test]
fn returned_origins_bind_every_recorded_identity_and_commitment() {
    for replay in [false, true] {
        for mutation in 0..5 {
            let fixture = Fixture::single();
            let mut returned = fixture.operation(now());
            match mutation {
                0 => returned.origin.case_id = CaseId::from_uuid(Uuid::from_u128(999)),
                1 => {
                    returned.origin.operation_id =
                        MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(999))
                }
                2 => returned.origin.submission_digest = Sha256Digest::from_array([99; 32]),
                3 => returned.origin.review_digest = Sha256Digest::from_array([99; 32]),
                _ => returned.origin.capture_digest = Sha256Digest::from_array([99; 32]),
            }
            reject_returned(fixture, returned, replay);
        }
    }
}

#[test]
fn coherently_rehashed_returned_rows_cannot_change_retained_or_administrative_facts() {
    for replay in [false, true] {
        for mutation in 0..5 {
            let fixture = Fixture::single();
            let mut returned = fixture.operation(now());
            match mutation {
                0 => returned.capture.records.clear(),
                1 => returned
                    .capture
                    .records
                    .push(returned.capture.records[0].clone()),
                2 => returned.capture.records[0].result.last_action = MeasureCaptureAction::Revoke,
                3 => {
                    returned.capture.records[0]
                        .result
                        .sources
                        .subject
                        .changed_by
                        .email = "Invented retained author".into()
                }
                _ => {
                    returned.capture.records[0].result.validity =
                        MeasureCaptureValidity::EnteredInError
                }
            }
            refresh(&mut returned);
            reject_returned(fixture, returned, replay);
        }
    }
}

#[test]
fn returned_operations_require_complete_exact_original_ancestor_closures() {
    for replay in [false, true] {
        for mutation in 0..5 {
            let fixture = Fixture::single();
            let mut returned = fixture.operation(now());
            let groups = &mut returned.record_history.records.judicial.groups;
            match mutation {
                0 => groups.clear(),
                1 => groups[0].origin.group_digest = Sha256Digest::from_array([99; 32]),
                2 => groups[0].capture.measures.clear(),
                3 => groups.push(groups[0].clone()),
                _ => {
                    let extra =
                        crate::measure_dependency_support::independent_request(9, 99).capture();
                    groups.extend(
                        crate::measure_dependency_support::judicial_inventory(&extra)
                            .records
                            .records
                            .judicial
                            .groups,
                    );
                }
            }
            reject_returned(fixture, returned, replay);
        }
    }
}
