use crate::{
    case_stage_database_support::FixedClock,
    deadline_backend_support::{history_query, snapshot, Fixture},
    deadline_tracked_backend_support::{canonical_evidence, stored_evidence},
};
use application::{deadline_reevaluation::encode_observations, deadlines::*, ApplicationError};
use domain::crypto::DocumentHasher;
use infrastructure::{PostgresDeadlineStore, RingSha256Hasher};
use std::sync::Arc;

/// Change only the manifest and its commitments under administrative access.
/// Every SQL constraint and generated projection remains installed and active.
pub fn write_coherent_tracking(db: &mut Fixture, forged: &mut DeadlineDetail) {
    let tracking = forged.tracking.as_ref().unwrap();
    let observations = encode_observations(&tracking.observations).unwrap();
    let tracking_bytes = deadline_tracking_capture_bytes(&RingSha256Hasher, tracking).unwrap();
    let DeadlineReceiptVersion::Tracked(metadata) = &mut forged.receipt.version else {
        panic!("tracked fixture required")
    };
    metadata.observations_digest = RingSha256Hasher.hash_bytes(&observations);
    let review = deadline_review_bytes(&RingSha256Hasher, forged).unwrap();
    let capture = deadline_capture_bytes(&RingSha256Hasher, forged).unwrap();
    forged.receipt.review_digest = RingSha256Hasher.hash_bytes(&review);
    forged.receipt.capture_digest = RingSha256Hasher.hash_bytes(&capture);
    let submission = deadline_record_submission_bytes(forged).unwrap();
    forged.receipt.submission_digest = RingSha256Hasher.hash_bytes(&submission);
    deadline_receipt_matches(&RingSha256Hasher, forged)
        .expect("forgery must be structurally valid and have consistent hashes");
    db.admin
        .batch_execute("ALTER TABLE case_deadline_revisions DISABLE TRIGGER USER")
        .unwrap();
    let changed = db.admin.execute(
        "UPDATE case_deadline_revisions SET
            observations_canonical=$3,tracking_canonical=$4,
            review_canonical=$5,review_digest=$6,capture_canonical=$7,capture_digest=$8,
            submission_canonical=$9,submission_digest=$10
         WHERE deadline_id=$1 AND revision=$2",
        &[
            &forged.id.as_uuid(),
            &i64::from(forged.revision.get()),
            &observations,
            &tracking_bytes,
            &review,
            &forged.receipt.review_digest.as_bytes().as_slice(),
            &capture,
            &forged.receipt.capture_digest.as_bytes().as_slice(),
            &submission,
            &forged.receipt.submission_digest.as_bytes().as_slice(),
        ],
    );
    db.admin
        .batch_execute("ALTER TABLE case_deadline_revisions ENABLE TRIGGER USER")
        .unwrap();
    assert_eq!(
        changed.expect("coherent forgery must satisfy all SQL checks"),
        1
    );
    assert_eq!(stored_evidence(db, forged), canonical_evidence(forged));
}

pub fn assert_rejected_by_reads_and_inventory(
    db: &mut Fixture,
    repository: &dyn DeadlineStore,
    forged: &DeadlineDetail,
) {
    let before = snapshot(db);
    for revision in [None, Some(forged.revision)] {
        assert!(matches!(
            repository.get(db.owner, db.case, forged.id, revision, db.at),
            Err(ApplicationError::Deadline(
                DeadlineError::StoredInconsistent(_)
            ))
        ));
    }
    assert!(matches!(
        repository.history(db.owner, db.case, forged.id, history_query(20, None), db.at),
        Err(ApplicationError::Deadline(
            DeadlineError::StoredInconsistent(_)
        ))
    ));
    let reopened = PostgresDeadlineStore::open(
        &db.runtime_url,
        Arc::new(RingSha256Hasher),
        Arc::new(FixedClock(db.at)),
    );
    match reopened {
        Err(ApplicationError::InvalidConfiguration(message)) => assert_eq!(
            message,
            "deadline inventory is inconsistent; restore a consistent database"
        ),
        Err(error) => panic!("expected inventory rejection, received {error}"),
        Ok(_) => panic!("inventory accepted unauthenticated tracking evidence"),
    }
    assert_eq!(snapshot(db), before);
}
