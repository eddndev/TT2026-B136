use super::*;
use axum::{body::Body, http::Request};
use domain::{crypto::DocumentHasher, DomainError};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tower::ServiceExt;

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

async fn rejects_before_projection(row: PrecautionaryHearingRecordStoredOperation) {
    let id = row.capture.review.command.hearing_id;
    let calls = Arc::new(AtomicUsize::new(0));
    let mut reads = MockRead::new();
    reads
        .expect_get()
        .times(1)
        .return_once(move |token, case, hearing, revision| {
            assert_eq!(
                (token, case, hearing, revision),
                ("staff-token", case_id(), id, None)
            );
            Ok(row)
        });
    let router = web::precautionary_hearing_router(
        Arc::new(MockContext::new()),
        Arc::new(MockWrite::new()),
        Arc::new(reads),
        Arc::new(CountHasher(calls.clone())),
    );
    let response = router
        .oneshot(
            Request::get(format!("{}/{id}", base()))
                .header("authorization", "Bearer staff-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status().as_u16(),
        500,
        "oversized returned history must not be projected"
    );
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "history shape must reject before projecting or hashing even the hearing prefix"
    );
}

#[tokio::test]
async fn regression_257_flat_owners_reject_before_any_response_projection() {
    let mut row = review_operation();
    let owner = row.history.record_history.records.judicial.groups[0].clone();
    row.history.record_history.records.judicial.groups = vec![owner; 257];
    rejects_before_projection(row).await;
}

#[tokio::test]
async fn regression_oversized_owned_group_arrays_reject_before_response_projection() {
    for fault in 0..3 {
        let mut row = review_operation();
        let group = &mut row.history.record_history.records.judicial.groups[0].capture;
        match fault {
            0 => group.measures = vec![group.measures[0].clone(); 33],
            1 => group.review.results = vec![group.review.results[0].clone(); 33],
            _ => {
                group.review.material.result_sources =
                    vec![group.review.material.result_sources[0].clone(); 33]
            }
        }
        rejects_before_projection(row).await;
    }
}
