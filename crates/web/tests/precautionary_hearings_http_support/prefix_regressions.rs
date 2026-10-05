use super::*;
use time::Duration;

#[tokio::test]
async fn regression_complete_257_revision_prefix_exceeds_the_transport_proof_bound() {
    let mut row = replaced();
    for revision in 3..=257 {
        let prior = row.history.captures.last().unwrap();
        let mut fixture = crate::receipt_support::Fixture::replace(prior);
        fixture.command.operation_id =
            PrecautionaryHearingOperationId::from_uuid(uuid::Uuid::from_u128(1000 + revision));
        let next = fixture.capture(
            Some(prior),
            crate::receipt_support::at() + Duration::seconds(revision as i64),
        );
        assert_eq!(next.review.result_revision.get(), revision as u32);
        row.history.captures.push(next);
    }
    row.capture = row.history.captures.last().unwrap().clone();
    assert_eq!(row.history.captures.len(), 257);
    assert_eq!(row.capture.review.result_revision.get(), 257);
    precautionary_hearing_history_with_decision_history_matches(
        &Hasher,
        &row.history.captures[..256],
        &row.history.origin,
        &row.history.record_history,
    )
    .unwrap();
    let id = row.capture.review.command.hearing_id;
    let mut read = MockRead::new();
    read.expect_get()
        .times(1)
        .return_once(move |_, _, _, _| Ok(row));
    let result = request(
        MockContext::new(),
        MockWrite::new(),
        read,
        "GET",
        &format!("{}/{id}", base()),
        None,
    )
    .await;
    assert_eq!(result.0, 500);
    assert_eq!(result.1, internal());
}

#[tokio::test]
async fn regression_foreign_case_or_hearing_inside_prefix_is_not_disclosed() {
    for foreign_case in [true, false] {
        let mut row = cancelled();
        let id = row.capture.review.command.hearing_id;
        let revision = &mut row.history.captures[1].review;
        if foreign_case {
            revision.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(99));
        } else {
            revision.command.hearing_id =
                PrecautionaryHearingId::from_uuid(uuid::Uuid::from_u128(99));
        }
        assert_eq!(row.history.captures.last(), Some(&row.capture));
        assert_eq!(row.history.origin.case_id, case_id());
        assert_eq!(row.history.origin.hearing_id, id);
        let mut read = MockRead::new();
        read.expect_get()
            .times(1)
            .return_once(move |_, _, _, _| Ok(row));
        let result = request(
            MockContext::new(),
            MockWrite::new(),
            read,
            "GET",
            &format!("{}/{id}", base()),
            None,
        )
        .await;
        assert_eq!(result.0, 500, "foreign_case={foreign_case}");
        assert_eq!(result.1, internal());
    }
}
