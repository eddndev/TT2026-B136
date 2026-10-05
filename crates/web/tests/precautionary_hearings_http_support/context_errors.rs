use super::*;
use application::cases::{case_administration_digest, CaseRevision};
use domain::case_administration::CaseAdministrativeStatus;

#[tokio::test]
async fn closed_context_is_readable_but_another_cases_context_is_not_disclosed() {
    let mut material = crate::context_support::initial();
    material.administration.revision = CaseRevision::new(2).unwrap();
    material.administration.values = material
        .administration
        .values
        .with_status(CaseAdministrativeStatus::Closed);
    material.administration.values_digest =
        case_administration_digest(&Hasher, &material.administration.values);
    let context = PrecautionaryContext::new(&Hasher, material).unwrap();
    let digest = context.digest(&Hasher).to_hex();
    let mut port = MockContext::new();
    port.expect_get()
        .times(1)
        .return_once(move |_, _| Ok(context));
    let (status, value) = request(
        port,
        MockWrite::new(),
        MockRead::new(),
        "GET",
        &context_path(),
        None,
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["context_digest"], digest);
    assert_eq!(value["administration"]["revision"], 2);
    assert_eq!(value["stage_administration"]["revision"], 1);
    assert_eq!(
        value["administration"]["changed_by"]["email"],
        "captured@example.com"
    );
    assert!(value["administration"]["changed_at"]
        .as_str()
        .unwrap()
        .contains(".123456789Z"));

    let mut material = crate::context_support::initial();
    let other = CaseId::from_uuid(uuid::Uuid::from_u128(99));
    material.case_id = other;
    material.administration.case_id = other;
    material.stage_administration.case_id = other;
    if let application::case_stages::CaseStageEntry::Initial(stage) = &mut material.stage {
        stage.case_id = other;
    }
    let other = PrecautionaryContext::new(&Hasher, material).unwrap();
    let mut port = MockContext::new();
    port.expect_get()
        .times(1)
        .return_once(move |_, _| Ok(other));
    let result = request(
        port,
        MockWrite::new(),
        MockRead::new(),
        "GET",
        &context_path(),
        None,
    )
    .await;
    assert_eq!(result, (500, internal()));
}

#[tokio::test]
async fn workflow_errors_preserve_auth_absence_conflicts_and_hide_stored_corruption() {
    for (error, expected, code) in [
        (ApplicationError::InvalidSession, 401, "invalid_session"),
        (ApplicationError::PermissionDenied, 403, "permission_denied"),
        (
            PrecautionaryHearingError::NotFound.into(),
            404,
            "precautionary_hearing_not_found",
        ),
        (
            PrecautionaryHearingError::OperationConflict.into(),
            409,
            "precautionary_hearing_operation_conflict",
        ),
        (
            PrecautionaryHearingError::SubmissionMismatch.into(),
            409,
            "precautionary_hearing_submission_mismatch",
        ),
        (
            PrecautionaryHearingError::ReviewMismatch.into(),
            409,
            "precautionary_hearing_review_mismatch",
        ),
        (
            PrecautionaryHearingError::IncompleteHistory.into(),
            422,
            "precautionary_hearing_incomplete_history",
        ),
        (
            PrecautionaryHearingError::StoredInconsistent("private SQL and actor detail".into())
                .into(),
            500,
            "internal_error",
        ),
    ] {
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |_, _, _, _| Err(error));
        let (status, value) = request(
            MockContext::new(),
            MockWrite::new(),
            read,
            "GET",
            &format!("{}/{}", base(), initial().capture.review.command.hearing_id),
            None,
        )
        .await;
        assert_eq!(status, expected, "{value}");
        assert_eq!(value["error"]["code"], code);
        if expected == 500 {
            assert_eq!(value, internal());
        }
    }
}
