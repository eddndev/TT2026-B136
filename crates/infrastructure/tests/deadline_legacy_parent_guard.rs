mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
#[allow(dead_code)]
mod deadline_tracked_notification_support;
mod procedural_fact_backend_support;

use application::{
    deadline_observations::build_deadline_observations,
    deadline_reevaluation::{encode_observations, ObservationRole},
    deadlines::*,
};
use deadline_backend_support as dl;
use deadline_tracked_backend_support as tracked;
use deadline_tracked_notification_support as notification;
use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;
use serde_json::json;

/// Try an attention successor without changing the predecessor or committing a success.
fn try_attention(db: &dl::Fixture, value: &DeadlineDetail) -> Result<(), postgres::Error> {
    deadline_receipt_matches(&RingSha256Hasher, value).unwrap();
    let tracking = value.tracking.as_ref().unwrap();
    let observations = encode_observations(&tracking.observations).unwrap();
    let suffix = deadline_tracking_capture_bytes(&RingSha256Hasher, tracking).unwrap();
    let review = deadline_review_bytes(&RingSha256Hasher, value).unwrap();
    let capture = deadline_capture_bytes(&RingSha256Hasher, value).unwrap();
    let submission = deadline_record_submission_bytes(value).unwrap();
    let DeadlineAttention::Recorded {
        occurred_at,
        statement,
        locator,
    } = &value.attention
    else {
        panic!("attention fixture required")
    };
    assert_eq!(
        *occurred_at,
        domain::procedural_time::DeclaredProceduralTime::unknown()
    );
    let attention = json!({"status":"recorded","occurred_at":{"precision":"unknown"},
        "statement":statement.as_str(),"locator":locator.as_str()});
    let mut client = db.runtime();
    let mut tx = client.transaction()?;
    let count = tx.execute("INSERT INTO case_deadline_revisions(
        deadline_id,case_id,revision,title,profile_id,profile_revision,input_canonical,result_canonical,
        observed_administration_revision,observed_administration_canonical,observed_administration_digest,
        responsible_id,responsible_email,responsible_role,attention,operation_id,action,reason,
        review_canonical,review_digest,capture_canonical,capture_digest,submission_canonical,submission_digest,
        recorded_at_seconds,recorded_at_nanoseconds,recorded_by,recorded_by_email,
        source_kind,source_id,source_revision,source_head_revision,source_hearing_id,
        source_parent_resolution_id,source_parent_resolution_revision,source_head_parent_resolution_revision,
        calendar_id,calendar_revision,calendar_head_revision,due_at_seconds,due_at_nanoseconds,
        tracking_canonical,observations_canonical)
        SELECT deadline_id,case_id,$2,title,profile_id,profile_revision,input_canonical,result_canonical,
        observed_administration_revision,observed_administration_canonical,observed_administration_digest,
        responsible_id,responsible_email,responsible_role,$3,$4,'set_attention',$5,$6,$7,$8,$9,$10,$11,
        $12,$13,$14,$15,source_kind,source_id,source_revision,source_head_revision,source_hearing_id,
        source_parent_resolution_id,source_parent_resolution_revision,source_head_parent_resolution_revision,
        calendar_id,calendar_revision,calendar_head_revision,due_at_seconds,due_at_nanoseconds,$16,$17
        FROM case_deadline_revisions WHERE deadline_id=$1 AND revision=$2::bigint-1",
        &[&value.id.as_uuid(), &i64::from(value.revision.get()), &attention,
        &value.receipt.operation_id.as_uuid(), &value.reason.as_ref().map(|reason| reason.as_str()),
        &review, &value.receipt.review_digest.as_bytes().as_slice(), &capture,
        &value.receipt.capture_digest.as_bytes().as_slice(), &submission,
        &value.receipt.submission_digest.as_bytes().as_slice(), &value.recorded_at.unix_timestamp(),
        &(value.recorded_at.nanosecond() as i32), &value.recorded_by.user_id().unwrap().as_uuid(),
        &value.recorded_by.email().unwrap(), &suffix, &observations])?;
    assert_eq!(count, 1);
    tx.rollback()
}

#[test]
fn legacy_attention_cannot_add_an_authentic_parent_head_that_the_original_never_observed() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (command, parent, _) = notification::setup(&db);
    let first = dl::persist_legacy(&db, db.owner, command);
    assert_eq!(first.receipt.version, DeadlineReceiptVersion::Legacy);
    let parent_head = notification::advance_parent(&db, &parent);
    let repository = dl::store(&db);
    let prepared =
        tracked::tracked_prepared(&db, repository.as_ref(), &dl::attention(&first), None);
    let mut candidate = tracked::prepared_detail(&db, &prepared);
    let original_observations = candidate.tracking.as_ref().unwrap().observations.clone();
    assert!(original_observations
        .entries
        .iter()
        .all(|entry| entry.role != ObservationRole::NotificationParent));
    let before = dl::snapshot(&mut db);
    try_attention(&db, &candidate).expect("ordinary legacy attention must pass the same SQL path");
    assert_eq!(dl::snapshot(&mut db), before);

    let observations = build_deadline_observations(
        &RingSha256Hasher,
        db.case,
        &first.calculation.profile,
        &first.calculation.material,
        Some(&parent_head),
    )
    .unwrap();
    assert_eq!(
        &observations.entries[..original_observations.entries.len()],
        original_observations.entries.as_slice()
    );
    assert_eq!(
        observations.entries.last().unwrap().role,
        ObservationRole::NotificationParent
    );
    assert_eq!(
        observations.entries.last().unwrap().revision,
        parent_head.snapshot.metadata().revision.get()
    );
    let manifest = encode_observations(&observations).unwrap();
    candidate.tracking.as_mut().unwrap().observations = observations;
    let DeadlineReceiptVersion::Tracked(metadata) = &mut candidate.receipt.version else {
        unreachable!()
    };
    metadata.observations_digest = RingSha256Hasher.hash_bytes(&manifest);
    candidate.receipt.review_digest =
        RingSha256Hasher.hash_bytes(&deadline_review_bytes(&RingSha256Hasher, &candidate).unwrap());
    candidate.receipt.capture_digest = RingSha256Hasher
        .hash_bytes(&deadline_capture_bytes(&RingSha256Hasher, &candidate).unwrap());
    candidate.receipt.submission_digest =
        RingSha256Hasher.hash_bytes(&deadline_record_submission_bytes(&candidate).unwrap());
    deadline_receipt_matches(&RingSha256Hasher, &candidate).unwrap();

    let error = try_attention(&db, &candidate)
        .expect_err("an authentic new parent is still a new observation");
    assert_eq!(
        error.code(),
        Some(&postgres::error::SqlState::CHECK_VIOLATION)
    );
    assert_eq!(dl::snapshot(&mut db), before);
}
