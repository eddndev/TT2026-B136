use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use domain::{crypto::DocumentHasher, DomainError};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tower::ServiceExt;

fn origin_mut(row: &mut MeasureDecisionRecordReceipt) -> &mut MeasureGroupOrigin {
    match row {
        MeasureDecisionRecordReceipt::V1(v) => &mut v.origin,
        MeasureDecisionRecordReceipt::V2(v) => &mut v.origin,
    }
}
async fn returned(row: MeasureDecisionRecordReceipt, id: MeasureDecisionId) -> (u16, Value) {
    let mut read = MockRead::new();
    read.expect_get()
        .times(1)
        .return_once(move |token, case, decision| {
            assert_eq!((token, case, decision), ("staff-token", case_id(), id));
            Ok(row)
        });
    request(
        MockWrite::new(),
        read,
        "GET",
        &format!("{}/{id}", base()),
        None,
    )
    .await
}

#[tokio::test]
async fn decision_detail_rejects_every_foreign_origin_selector_or_commitment() {
    for original in [g1(), g2()] {
        let id = original.origin().decision_id;
        for fault in 0..7 {
            let mut wrong = original.clone();
            let origin = origin_mut(&mut wrong);
            let digest = Sha256Digest::from_array([254; 32]);
            match fault {
                0 => origin.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(999)),
                1 => {
                    origin.operation_id =
                        MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(999))
                }
                2 => origin.decision_id = MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(999)),
                3 => origin.submission_digest = digest,
                4 => origin.review_digest = digest,
                5 => origin.decision_digest = digest,
                _ => origin.group_digest = digest,
            }
            let (status, body) = returned(wrong, id).await;
            assert_eq!(status, 500, "fault {fault}: {body}");
            assert_eq!(body, internal());
        }
    }
}

#[tokio::test]
async fn operation_lookup_rejects_a_different_genuine_original_operation() {
    let expected = g1();
    let returned = g2();
    let operation = expected.origin().operation_id;
    let mut read = MockRead::new();
    read.expect_get_operation()
        .times(1)
        .return_once(move |token, case, op| {
            assert_eq!((token, case, op), ("staff-token", case_id(), operation));
            Ok(returned)
        });
    let (status, body) = request(
        MockWrite::new(),
        read,
        "GET",
        &format!("{}/operations/{operation}", base()),
        None,
    )
    .await;
    assert_eq!(status, 500, "{body}");
    assert_eq!(body, internal());
}

#[tokio::test]
async fn selected_decision_and_measure_rows_bind_the_actual_group_owner() {
    for fault in 0..4 {
        let mut row = g2();
        let id = row.origin().decision_id;
        let MeasureDecisionRecordReceipt::V2(v) = &mut row else {
            unreachable!()
        };
        match fault {
            0 => v.group.decision.case_id = CaseId::from_uuid(uuid::Uuid::from_u128(999)),
            1 => {
                v.group.decision.operation_id =
                    MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(999))
            }
            2 => {
                v.group.measures[0].decision_id =
                    MeasureDecisionId::from_uuid(uuid::Uuid::from_u128(999))
            }
            _ => v.group.measures[0].decision_digest = Sha256Digest::from_array([254; 32]),
        }
        let (status, body) = returned(row, id).await;
        assert_eq!(status, 500, "fault {fault}: {body}");
        assert_eq!(body, internal());
    }
}

#[tokio::test]
async fn prepare_rejects_changed_normalized_command_or_case_in_either_family() {
    for original in [g1(), g2()] {
        for foreign_case in [false, true] {
            let input = original.command().clone();
            let body = command(&input);
            let mut wrong = review(&original);
            let (case, command) = match &mut wrong {
                MeasureDecisionRecordReview::V1(v) => (&mut v.case_id, &mut v.command),
                MeasureDecisionRecordReview::V2(v) => (&mut v.case_id, &mut v.command),
            };
            if foreign_case {
                *case = CaseId::from_uuid(uuid::Uuid::from_u128(999));
            } else {
                command.operation_id =
                    MeasureDecisionOperationId::from_uuid(uuid::Uuid::from_u128(999));
            }
            let mut write = MockWrite::new();
            write
                .expect_prepare()
                .times(1)
                .return_once(move |token, case, command| {
                    assert_eq!((token, case, command), ("staff-token", case_id(), input));
                    Ok(wrong)
                });
            let (status, body) = request(
                write,
                MockRead::new(),
                "POST",
                &format!("{}/prepare", base()),
                Some(body),
            )
            .await;
            assert_eq!(status, 500, "{body}");
            assert_eq!(body, internal());
        }
    }
}

#[tokio::test]
async fn submit_rejects_either_returned_confirmation_even_when_origin_and_review_agree() {
    for original in [g1(), g2()] {
        for instruction in [true, false] {
            let body = submission(&original);
            let mut wrong = original.clone();
            let digest = Sha256Digest::from_array([254; 32]);
            let origin = origin_mut(&mut wrong);
            if instruction {
                origin.submission_digest = digest;
            } else {
                origin.review_digest = digest;
            }
            let (submission, review) = match &mut wrong {
                MeasureDecisionRecordReceipt::V1(v) => (
                    &mut v.group.review.submission_digest,
                    &mut v.group.review.review_digest,
                ),
                MeasureDecisionRecordReceipt::V2(v) => (
                    &mut v.group.review.submission_digest,
                    &mut v.group.review.review_digest,
                ),
            };
            if instruction {
                *submission = digest;
            } else {
                *review = digest;
            }
            let original = original.clone();
            let mut write = MockWrite::new();
            write.expect_submit().times(1).return_once(
                move |token, case, command, confirmation| {
                    assert_eq!(
                        (token, case, command),
                        ("staff-token", case_id(), original.command().clone())
                    );
                    assert_eq!(confirmation.submission_digest, original.submission_digest());
                    assert_eq!(confirmation.review_digest, original.review_digest());
                    Ok(wrong)
                },
            );
            let (status, body) = request(
                write,
                MockRead::new(),
                "POST",
                &format!("{}/submit", base()),
                Some(body),
            )
            .await;
            assert_eq!(status, 500, "{body}");
            assert_eq!(body, internal());
        }
    }
}

struct CountHasher(Arc<AtomicUsize>);
impl DocumentHasher for CountHasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_bytes(bytes)
    }
    fn hash_stream(&self, reader: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Hasher.hash_stream(reader)
    }
}
async fn oversized(row: MeasureDecisionRecordReceipt) {
    let id = row.origin().decision_id;
    let calls = Arc::new(AtomicUsize::new(0));
    let mut read = MockRead::new();
    read.expect_get()
        .times(1)
        .return_once(move |_, _, _| Ok(row));
    let response = web::measure_decision_router(
        Arc::new(MockWrite::new()),
        Arc::new(read),
        Arc::new(CountHasher(calls.clone())),
    )
    .oneshot(
        Request::get(format!("{}/{id}", base()))
            .header("authorization", "Bearer staff-token")
            .body(Body::empty())
            .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(response.status().as_u16(), 500);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "proof size must reject before hashing any projected context"
    );
    let bytes = to_bytes(response.into_body(), 1024).await.unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), internal());
}

#[tokio::test]
async fn stored_proof_budget_includes_current_owner_before_any_projection_hash() {
    let mut row = g1();
    let MeasureDecisionRecordReceipt::V1(v) = &mut row else {
        unreachable!()
    };
    v.measure_history.groups = vec![
        MeasureGroupEvidence {
            origin: v.origin.clone(),
            capture: v.group.clone()
        };
        256
    ];
    oversized(row).await;
    let mut row = g2();
    let MeasureDecisionRecordReceipt::V2(v) = &mut row else {
        unreachable!()
    };
    v.record_history.records.judicial.groups =
        vec![v.record_history.records.judicial.groups[0].clone(); 256];
    v.record_history.records.administrative.clear();
    oversized(row).await;
}

#[tokio::test]
async fn oversized_current_group_and_ancestor_arrays_reject_before_any_projection_hash() {
    for fault in 0..4 {
        let mut row = g2();
        let MeasureDecisionRecordReceipt::V2(v) = &mut row else {
            unreachable!()
        };
        match fault {
            0 => v.group.measures = vec![v.group.measures[0].clone(); 33],
            1 => v.group.review.results = vec![v.group.review.results[0].clone(); 33],
            2 => {
                v.group.review.material.predecessors =
                    vec![v.group.review.material.predecessors[0].clone(); 33]
            }
            _ => {
                let ancestor = &mut v.record_history.records.judicial.groups[0].capture;
                ancestor.review.material.result_sources =
                    vec![ancestor.review.material.result_sources[0].clone(); 33];
            }
        }
        oversized(row).await;
    }
}
