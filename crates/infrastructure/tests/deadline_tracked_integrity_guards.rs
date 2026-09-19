mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_guard_support;
mod procedural_fact_backend_support;

use application::{
    deadline_reevaluation::encode_observations, deadline_tracking::TrackingPolicy, deadlines::*,
};
use deadline_backend_support::*;
use deadline_tracked_backend_support::*;
use deadline_tracked_guard_support::*;
use domain::crypto::{DocumentHasher, Sha256Digest};
use infrastructure::RingSha256Hasher;

fn resign(value: &mut DeadlineDetail) {
    let observations = encode_observations(&value.tracking.as_ref().unwrap().observations).unwrap();
    let DeadlineReceiptVersion::Tracked(metadata) = &mut value.receipt.version else {
        panic!("tracked fixture required")
    };
    metadata.observations_digest = RingSha256Hasher.hash_bytes(&observations);
    value.receipt.review_digest =
        RingSha256Hasher.hash_bytes(&deadline_review_bytes(&RingSha256Hasher, value).unwrap());
    value.receipt.capture_digest =
        RingSha256Hasher.hash_bytes(&deadline_capture_bytes(&RingSha256Hasher, value).unwrap());
    value.receipt.submission_digest =
        RingSha256Hasher.hash_bytes(&deadline_record_submission_bytes(value).unwrap());
    deadline_receipt_matches(&RingSha256Hasher, value).unwrap();
}

#[test]
fn sql_rejects_each_forged_predecessor_digest_even_after_resigning_the_successor() {
    let Some(mut db) = Fixture::new() else { return };
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &setup(&db), Some(fixed_profile()));
    let first = prepared_detail(&db, &prepared);
    insert(&db, &first).unwrap();
    for change_capture in [false, true] {
        let prepared = tracked_prepared(&db, repository.as_ref(), &attention(&first), None);
        let mut changed = prepared_detail(&db, &prepared);
        let DeadlineReceiptVersion::Tracked(metadata) = &mut changed.receipt.version else {
            unreachable!()
        };
        let predecessor = metadata.predecessor.as_mut().unwrap();
        if change_capture {
            predecessor.capture_digest = Sha256Digest::from_array([0x91; 32]);
        } else {
            predecessor.submission_digest = Sha256Digest::from_array([0x92; 32]);
        }
        resign(&mut changed);
        let before = snapshot(&mut db);
        let error =
            insert(&db, &changed).expect_err("exact predecessor receipts must survive direct SQL");
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
        assert_eq!(snapshot(&mut db), before);
    }
}

#[test]
fn sql_attention_cannot_change_a_declared_policy_with_otherwise_valid_receipts() {
    let Some(mut db) = Fixture::new() else { return };
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &setup(&db), Some(fixed_profile()));
    let first = prepared_detail(&db, &prepared);
    insert(&db, &first).unwrap();
    let prepared = tracked_prepared(&db, repository.as_ref(), &attention(&first), None);
    let mut changed = prepared_detail(&db, &prepared);
    changed.tracking.as_mut().unwrap().policies.profile = TrackingPolicy::Follow;
    resign(&mut changed);
    let before = snapshot(&mut db);
    let error = insert(&db, &changed).expect_err("attention is not a new policy qualification");
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn sql_registration_compares_observed_receipts_to_the_exact_persisted_revisions() {
    let Some(mut db) = Fixture::new() else { return };
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &setup(&db), Some(fixed_profile()));
    let original = prepared_detail(&db, &prepared);
    for index in [0, 1] {
        let mut changed = original.clone();
        changed.tracking.as_mut().unwrap().observations.entries[index].submission_digest =
            Sha256Digest::from_array([0x93; 32]);
        resign(&mut changed);
        let before = snapshot(&mut db);
        let error = insert(&db, &changed)
            .expect_err("DLOB receipt must match its actual profile or source revision");
        assert_eq!(
            error.code(),
            Some(&postgres::error::SqlState::CHECK_VIOLATION)
        );
        assert_eq!(snapshot(&mut db), before);
    }
}
