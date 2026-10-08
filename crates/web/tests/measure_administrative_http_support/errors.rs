use super::*;

#[tokio::test]
async fn bearer_is_required_before_command_and_read_ports() {
    for (method, path, body) in [
        (
            "POST",
            prepare_path(),
            command_json(&correct().capture.review).to_string(),
        ),
        ("GET", base(), String::new()),
        ("GET", operation_path(&correct()), String::new()),
    ] {
        let (status, response) = raw(
            (MockWrite::new(), MockRead::new(), MockRecords::new()),
            method,
            &path,
            "",
            body,
            "application/json",
        )
        .await;
        assert_eq!(status, 401, "{response}");
    }
}

#[tokio::test]
async fn command_workflow_errors_keep_stable_status_and_opaque_storage_details() {
    use MeasureAdministrativeError as A;
    let cases = [
        (ApplicationError::InvalidSession, 401, None),
        (ApplicationError::PermissionDenied, 403, None),
        (
            ApplicationError::from(A::OperationConflict),
            409,
            Some("measure_administrative_operation_conflict"),
        ),
        (
            ApplicationError::from(A::SubmissionMismatch),
            409,
            Some("measure_administrative_submission_mismatch"),
        ),
        (
            ApplicationError::from(A::ReviewMismatch),
            409,
            Some("measure_administrative_review_mismatch"),
        ),
        (
            ApplicationError::from(A::StaleHead),
            409,
            Some("measure_administrative_stale_head"),
        ),
        (
            ApplicationError::from(A::KnownDependants),
            409,
            Some("measure_administrative_known_dependants"),
        ),
        (
            ApplicationError::from(A::IncompleteHistory),
            422,
            Some("measure_administrative_incomplete_history"),
        ),
        (
            ApplicationError::InvalidInput("Invalid recorded terms".into()),
            422,
            None,
        ),
        (
            ApplicationError::from(A::StoredInconsistent("private SQL payload".into())),
            500,
            Some("internal_error"),
        ),
    ];
    for (error, expected, code) in cases {
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(move |_, _, _, _| Err(error));
        let (status, body) = request(
            write,
            MockRead::new(),
            "POST",
            &submit_path(),
            Some(submit_json(&correct())),
        )
        .await;
        assert_eq!(status, expected, "{body}");
        if let Some(code) = code {
            assert_eq!(body["error"]["code"], code);
        }
        if expected == 500 {
            assert_eq!(body, internal());
        }
        assert!(!body.to_string().contains("private SQL payload"));
    }
}

#[tokio::test]
async fn operation_absence_is_404_while_original_receipt_corruption_is_opaque_500() {
    for (error, expected) in [
        (MeasureAdministrativeError::NotFound, 404),
        (
            MeasureAdministrativeError::StoredInconsistent("audit sequence 123".into()),
            500,
        ),
    ] {
        let mut read = MockRead::new();
        read.expect_get_operation()
            .times(1)
            .return_once(move |_, _, _| Err(error.into()));
        let (status, body) = request(
            MockWrite::new(),
            read,
            "GET",
            &operation_path(&correct()),
            None,
        )
        .await;
        assert_eq!(status, expected, "{body}");
        if expected == 500 {
            assert_eq!(body, internal());
        } else {
            assert_eq!(body["error"]["code"], "measure_administrative_not_found");
        }
    }
}
