use super::*;

async fn current_result(
    expected: &MeasureRecordDetail,
    returned: MeasureRecordDetail,
) -> (u16, Value) {
    let mut port = MockRecords::new();
    let (case, id) = (expected.case_id, expected.reference.id());
    port.expect_get().times(1).return_once(move |token, c, i| {
        assert_eq!((token, c, i), ("staff-token", case, id));
        Ok(returned)
    });
    request(port, &current_path(id)).await
}
async fn exact_result(
    expected: &MeasureRecordDetail,
    returned: MeasureRecordDetail,
) -> (u16, Value) {
    let mut port = MockRecords::new();
    let (case, reference) = (expected.case_id, expected.reference);
    port.expect_exact()
        .times(1)
        .return_once(move |token, c, r| {
            assert_eq!((token, c, r), ("staff-token", case, reference));
            Ok(returned)
        });
    request(port, &exact_path(reference)).await
}
fn different_reference(value: PrecautionaryMeasureRef, field: u8) -> PrecautionaryMeasureRef {
    PrecautionaryMeasureRef::new(
        if field == 0 { id(999) } else { value.id() },
        if field == 1 {
            MeasureRevision::new(value.revision().get() + 1).unwrap()
        } else {
            value.revision()
        },
        if field == 2 {
            Sha256Digest::from_array([255; 32])
        } else {
            value.digest()
        },
    )
}

#[tokio::test]
async fn current_output_rejects_foreign_case_identity_and_selected_capture_mismatch() {
    let expected = initial(1);
    let mut cases = Vec::new();
    let mut wrong = expected.clone();
    wrong.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    cases.push(wrong);
    for field in 0..3 {
        let mut wrong = expected.clone();
        wrong.reference = different_reference(wrong.reference, field);
        cases.push(wrong);
    }
    let mut wrong = expected.clone();
    if let OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(v)) = &mut wrong.record {
        v.capture.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    }
    cases.push(wrong);
    for returned in cases {
        let (status, body) = current_result(&expected, returned).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn exact_output_binds_requested_case_identity_revision_and_digest() {
    let expected = mixed(2).remove(2);
    let mut cases = Vec::new();
    let mut wrong = expected.clone();
    wrong.case_id = CaseId::from_uuid(Uuid::from_u128(999));
    cases.push(wrong);
    for field in 0..3 {
        let mut wrong = expected.clone();
        wrong.reference = different_reference(wrong.reference, field);
        cases.push(wrong);
    }
    for returned in cases {
        let (status, body) = exact_result(&expected, returned).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn record_output_requires_its_actual_owner_in_the_complete_proof_for_each_family() {
    for expected in mixed(3).into_iter().take(3) {
        let mut wrong = expected.clone();
        match &mut wrong.record {
            OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V1(v)) => {
                v.owner.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999));
            }
            OwnedMeasureRecord::Judicial(OwnedJudicialMeasure::V2(v)) => {
                v.owner.operation_id = MeasureDecisionOperationId::from_uuid(Uuid::from_u128(999));
            }
            OwnedMeasureRecord::Administrative { owner, .. } => {
                owner.operation_id = MeasureCorrectionOperationId::from_uuid(Uuid::from_u128(999));
            }
        }
        let (status, body) = current_result(&expected, wrong).await;
        assert_eq!(status, 500, "{body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn record_output_rejects_lost_selected_owner_or_missing_selected_sibling() {
    let expected = initial(1);
    let mut missing = expected.clone();
    missing.record_history.records.judicial.groups.clear();
    let (status, body) = current_result(&expected, missing).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body, internal());
    let mut missing = expected.clone();
    missing.record_history.records.judicial.groups[0]
        .capture
        .measures
        .clear();
    let (status, body) = current_result(&expected, missing).await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body, internal());
}
