use super::*;
use domain::crypto::DocumentHasher;

fn refresh(value: &mut MeasureAdministrativeStoredOperation) {
    let capture = &mut value.capture;
    capture.review.review_digest =
        Hasher.hash_bytes(&measure_administrative_review_bytes(&capture.review).unwrap());
    for row in &mut capture.records {
        row.review_digest = capture.review.review_digest;
        row.capture_digest = Hasher.hash_bytes(&measure_administrative_record_bytes(row).unwrap());
    }
    capture.capture_digest =
        Hasher.hash_bytes(&measure_administrative_capture_bytes(capture).unwrap());
    value.origin.review_digest = capture.review.review_digest;
    value.origin.capture_digest = capture.capture_digest;
}

#[test]
fn every_read_binds_every_original_administrative_origin_field() {
    let original = operation(10);
    for mutation in 0..5 {
        let mut returned = original.clone();
        match mutation {
            0 => returned.origin.case_id = crate::participant_support::other_case(),
            1 => returned.origin.operation_id = operation_id(99),
            2 => returned.origin.submission_digest = Sha256Digest::from_array([99; 32]),
            3 => returned.origin.review_digest = Sha256Digest::from_array([99; 32]),
            _ => returned.origin.capture_digest = Sha256Digest::from_array([99; 32]),
        }
        reject_reads(&original, &returned);
    }
}

#[test]
fn each_read_reconstructs_rows_and_retained_facts_even_with_rehashed_claims() {
    let original = operation(10);
    for mutation in 0..7 {
        let mut returned = original.clone();
        match mutation {
            0 => returned.capture.records.clear(),
            1 => returned
                .capture
                .records
                .push(returned.capture.records[0].clone()),
            2 => returned.capture.records[0].result.last_action = MeasureCaptureAction::Revoke,
            3 => {
                returned.capture.records[0].result.validity = MeasureCaptureValidity::EnteredInError
            }
            4 => {
                returned.capture.records[0]
                    .result
                    .sources
                    .subject
                    .changed_by
                    .email = "invented@example.test".into()
            }
            5 => {
                returned.capture.records[0]
                    .result
                    .projection
                    .subject
                    .display_name = "Invented subject".into()
            }
            _ => returned.capture.records[0].actor.email = "invented@example.test".into(),
        }
        refresh(&mut returned);
        reject_reads(&original, &returned);
    }
}

#[test]
fn every_read_requires_the_exact_complete_original_closure() {
    let original = operation(10);
    for mutation in 0..5 {
        let mut returned = original.clone();
        let groups = &mut returned.record_history.records.judicial.groups;
        match mutation {
            0 => groups.clear(),
            1 => groups[0].origin.group_digest = Sha256Digest::from_array([99; 32]),
            2 => groups[0].capture.measures.clear(),
            3 => groups.push(groups[0].clone()),
            _ => groups.extend(operation(20).record_history.records.judicial.groups),
        }
        reject_reads(&original, &returned);
    }
}

#[test]
fn a_valid_receipt_from_another_case_cannot_be_disclosed_as_the_requested_case() {
    let original = operation(10);
    let actor = reader(Role::Paralegal);
    let case_id = crate::participant_support::other_case();
    for kind in READS {
        let returned = original.clone();
        let mut store = MockReads::new();
        match kind {
            ReadKind::List => {
                store
                    .expect_list()
                    .times(1)
                    .return_once(move |_, _, _| Ok(page(case_id, vec![returned])));
                assert!(service(store, identity(&actor))
                    .list(
                        "session",
                        case_id,
                        MeasureAdministrativeReadQuery::default()
                    )
                    .is_err());
            }
            ReadKind::Operation => {
                store
                    .expect_get_operation()
                    .times(1)
                    .return_once(move |_, _, _| Ok(returned));
                assert!(service(store, identity(&actor))
                    .get_operation("session", case_id, original.origin.operation_id)
                    .is_err());
            }
        }
    }
}
