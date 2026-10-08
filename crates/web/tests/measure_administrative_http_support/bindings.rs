use super::*;

#[tokio::test]
async fn submission_and_review_confirmations_are_independent_port_arguments_and_output_bindings() {
    for submission in [true, false] {
        let operation = correct();
        let mut body = submit_json(&operation);
        let mut expected = confirm(&operation);
        let digest = Sha256Digest::from_array([77; 32]);
        if submission {
            body["expected_submission_digest"] = json!(digest.to_hex());
            expected.submission_digest = digest;
        } else {
            body["expected_review_digest"] = json!(digest.to_hex());
            expected.review_digest = digest;
        }
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(move |_, _, _, received| {
                assert_eq!(received, expected);
                Ok(operation)
            });
        let (status, response) =
            request(write, MockRead::new(), "POST", &submit_path(), Some(body)).await;
        assert_eq!(status, 500, "{response}");
        assert_eq!(response, internal());
    }
}

#[tokio::test]
async fn prepare_rejects_wrong_returned_case_or_normalized_command() {
    for kind in 0..4 {
        let mut review = correct().capture.review;
        let body = command_json(&review);
        match kind {
            0 => review.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(99)),
            1 => {
                review.command.operation_id =
                    MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(999))
            }
            2 => review.command.reason = crate::correction_support::note("Different instruction"),
            _ => review.command.action = MeasureAdministrativeAction::MarkEnteredInError,
        }
        let mut write = MockWrite::new();
        write
            .expect_prepare()
            .times(1)
            .return_once(move |_, _, _| Ok(review));
        let (status, response) =
            request(write, MockRead::new(), "POST", &prepare_path(), Some(body)).await;
        assert_eq!(status, 500, "{kind}: {response}");
        assert_eq!(response, internal());
    }
}

#[tokio::test]
async fn operation_response_binds_origin_case_operation_and_all_three_commitments() {
    for kind in 0..6 {
        let mut operation = correct();
        let path = operation_path(&operation);
        match kind {
            0 => operation.origin.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(99)),
            1 => {
                operation.origin.operation_id =
                    MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(999))
            }
            2 => operation.origin.submission_digest = Sha256Digest::from_array([2; 32]),
            3 => operation.origin.review_digest = Sha256Digest::from_array([3; 32]),
            4 => operation.origin.capture_digest = Sha256Digest::from_array([4; 32]),
            _ => {
                operation.capture.review.command.operation_id =
                    MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(998))
            }
        }
        let mut read = MockRead::new();
        read.expect_get_operation()
            .times(1)
            .return_once(move |_, _, _| Ok(operation));
        let (status, body) = request(MockWrite::new(), read, "GET", &path, None).await;
        assert_eq!(status, 500, "{kind}: {body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn replacement_response_rejects_lost_sibling_wrong_row_scope_and_role_link() {
    for kind in 0..7 {
        let mut operation = replacement();
        let path = operation_path(&operation);
        match kind {
            0 => {
                operation.capture.records.pop();
            }
            1 => {
                operation.capture.records[0].case_id = CaseId::from_uuid(uuid::Uuid::from_u128(99))
            }
            2 => {
                operation.capture.records[0].operation_id =
                    MeasureCorrectionOperationId::from_uuid(uuid::Uuid::from_u128(999))
            }
            3 => operation.capture.records[0].result.id = crate::correction_support::id(888),
            4 => operation.capture.replacement_link = None,
            5 => {
                let link = operation.capture.replacement_link.as_mut().unwrap();
                std::mem::swap(&mut link.entered_in_error, &mut link.replacement);
            }
            _ => {
                let link = operation.capture.replacement_link.as_mut().unwrap();
                link.replacement = PrecautionaryMeasureRef::new(
                    link.replacement.id(),
                    link.replacement.revision(),
                    Sha256Digest::from_array([7; 32]),
                );
            }
        }
        let mut read = MockRead::new();
        read.expect_get_operation()
            .times(1)
            .return_once(move |_, _, _| Ok(operation));
        let (status, body) = request(MockWrite::new(), read, "GET", &path, None).await;
        assert_eq!(status, 500, "{kind}: {body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn over_budget_returned_history_is_rejected_without_truncation() {
    let mut operation = correct();
    let entry = operation.record_history.records.judicial.groups[0].clone();
    operation.record_history.records.judicial.groups = vec![entry; 257];
    let (status, body) = get_json(operation).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body, internal());
}
