use super::*;

#[tokio::test]
async fn preparation_preserves_schedule_replace_cancel_and_both_commitments() {
    for row in [initial(), replaced(), cancelled()] {
        let expected = row.capture.review.clone();
        let input = command(&expected.command);
        let returned = expected.clone();
        let mut write = MockWrite::new();
        write
            .expect_prepare()
            .times(1)
            .return_once(move |token, case, command| {
                assert_eq!((token, case), ("staff-token", case_id()));
                assert_eq!(command, returned.command);
                Ok(returned)
            });
        let (status, value) = request(
            MockContext::new(),
            write,
            MockRead::new(),
            "POST",
            &format!("{}/prepare", base()),
            Some(input.clone()),
        )
        .await;
        assert_eq!(status, 200, "{value}");
        assert_eq!(value["command"], input);
        assert_eq!(value["result_revision"], expected.result_revision.get());
        assert_eq!(
            value["status"],
            if expected.result_revision.get() == 3 {
                "cancelled"
            } else {
                "scheduled"
            }
        );
        assert_eq!(value["resolved_values"]["purpose"], "imposition");
        assert_eq!(
            value["resolved_values"]["scheduling_basis"]["locator"],
            "Page 1"
        );
        assert_eq!(
            value["actor"],
            json!({"id":"00000000-0000-0000-0000-000000000002",
            "email":"recording@example.test","role":"litigator"})
        );
        assert_eq!(value["sources"]["support"]["name"], "appointment.pdf");
        assert_eq!(value["sources"]["support"]["format"], "pdf");
        assert_eq!(value["participants"].as_array().unwrap().len(), 2);
        assert_eq!(
            value["submission_digest"],
            expected.submission_digest.to_hex()
        );
        assert_eq!(value["review_digest"], expected.review_digest.to_hex());
        assert_eq!(
            value["scheduling_context"]["administration"]["revision"],
            expected
                .scheduling_context
                .material()
                .administration
                .revision
                .get()
        );
    }
}

#[tokio::test]
async fn submit_binds_both_digests_and_returns_original_complete_prefix_on_replay() {
    let row = cancelled();
    let mut previous = None;
    for _ in 0..2 {
        let returned = row.clone();
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(move |token, case, input, confirmation| {
                assert_eq!((token, case), ("staff-token", case_id()));
                assert_eq!(input, returned.capture.review.command);
                assert_eq!(
                    confirmation.submission_digest,
                    returned.capture.review.submission_digest
                );
                assert_eq!(
                    confirmation.review_digest,
                    returned.capture.review.review_digest
                );
                Ok(returned)
            });
        let (status, value) = request(
            MockContext::new(),
            write,
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(submission(&row)),
        )
        .await;
        assert_eq!(status, 201, "{value}");
        assert_eq!(value["capture"]["review"]["status"], "cancelled");
        assert_eq!(
            value["capture"]["capture_digest"],
            row.capture.capture_digest.to_hex()
        );
        assert!(value["capture"]["recorded_at"]
            .as_str()
            .unwrap()
            .contains(".123456789Z"));
        assert_eq!(value["history"]["captures"].as_array().unwrap().len(), 3);
        assert_eq!(
            value["history"]["captures"][0]["review"]["result_revision"],
            1
        );
        assert_eq!(value["history"]["captures"][2], value["capture"]);
        assert_eq!(
            value["history"]["origin"]["operation_id"],
            "00000000-0000-0000-0000-00000000001e"
        );
        if let Some(prior) = previous {
            assert_eq!(value, prior);
        }
        previous = Some(value);
    }
}

#[tokio::test]
async fn review_targets_and_actual_owner_history_survive_http_projection() {
    let row = review_operation();
    let expected = row.capture.review.clone();
    let returned = expected.clone();
    let mut write = MockWrite::new();
    write
        .expect_prepare()
        .times(1)
        .return_once(move |_, _, command| {
            assert_eq!(command, returned.command);
            Ok(returned)
        });
    let (status, value) = request(
        MockContext::new(),
        write,
        MockRead::new(),
        "POST",
        &format!("{}/prepare", base()),
        Some(command(&expected.command)),
    )
    .await;
    assert_eq!(status, 200, "{value}");
    let target = expected.resolved_values.review_targets()[0];
    assert_eq!(
        value["resolved_values"]["review_targets"],
        json!([{
            "id":"00000000-0000-0000-0000-000000000046","revision":1,
            "capture_digest":target.digest().to_hex(),
        }])
    );
    let mut read = MockRead::new();
    let returned = row.clone();
    read.expect_get_operation()
        .times(1)
        .return_once(move |_, _, _| Ok(returned));
    let (status, value) = request(
        MockContext::new(),
        MockWrite::new(),
        read,
        "GET",
        &format!(
            "{}/operations/{}",
            base(),
            row.capture.review.command.operation_id
        ),
        None,
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(
        value["history"]["record_history"]["records"]["judicial"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        value["history"]["record_history"]["records"]["administrative"],
        json!([])
    );
    assert_eq!(value["history"]["record_history"]["decisions"], json!([]));
}

#[tokio::test]
async fn returned_command_or_confirmation_mismatch_is_not_disclosed() {
    for fault in 0..3 {
        let expected = initial();
        let mut returned = expected.clone();
        match fault {
            0 => {
                returned.capture.review.command.operation_id =
                    PrecautionaryHearingOperationId::new()
            }
            1 => returned.capture.review.submission_digest = Sha256Digest::from_array([71; 32]),
            _ => returned.capture.review.review_digest = Sha256Digest::from_array([72; 32]),
        }
        let mut write = MockWrite::new();
        write
            .expect_submit()
            .times(1)
            .return_once(move |_, _, _, _| Ok(returned));
        let result = request(
            MockContext::new(),
            write,
            MockRead::new(),
            "POST",
            &format!("{}/submit", base()),
            Some(submission(&expected)),
        )
        .await;
        assert_eq!(result, (500, internal()), "fault {fault}");
    }
}
