#![allow(dead_code)]
use crate::deadline_backend_support::{history_query, Fixture};
use application::{
    deadline_tracking::{TrackingPolicies, TrackingPolicy},
    deadlines::*,
};
use infrastructure::RingSha256Hasher;

pub fn policies() -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Follow,
        source: TrackingPolicy::Follow,
        calendar: TrackingPolicy::Undetermined,
    }
}

pub fn tracked_prepared(
    db: &Fixture,
    store: &dyn DeadlineStore,
    command: &DeadlineCommand,
    policies: Option<TrackingPolicies>,
) -> PreparedDeadlineChange {
    let preparation = store.prepare(db.owner, db.case, command).unwrap();
    prepare_tracked_deadline_change(
        &RingSha256Hasher,
        DeadlineActorSnapshot::User {
            id: db.owner,
            email: "owner@example.test".into(),
        },
        db.case,
        command.clone(),
        preparation,
        policies,
        None,
    )
    .expect("persisted profile and resolution must produce a valid tracked preparation")
}

pub fn prepared_detail(db: &Fixture, value: &PreparedDeadlineChange) -> DeadlineDetail {
    DeadlineDetail {
        id: value.command().deadline_id,
        case_id: value.case_id(),
        revision: value.command().result_revision().unwrap(),
        definition: value.definition().clone(),
        calculation: value.calculation().clone(),
        tracking: value.tracking().cloned(),
        responsible: value.responsible().clone(),
        attention: value.attention().clone(),
        status: value.status(),
        reason: value.command().reason().cloned(),
        receipt: value.receipt(),
        recorded_at: db.at.to_offset(time::UtcOffset::UTC),
        recorded_by: value.tracked_author().unwrap().clone(),
    }
}

pub fn canonical_evidence(value: &DeadlineDetail) -> Vec<Vec<u8>> {
    vec![
        deadline_review_bytes(&RingSha256Hasher, value).unwrap(),
        deadline_capture_bytes(&RingSha256Hasher, value).unwrap(),
        deadline_record_submission_bytes(value).unwrap(),
        value.receipt.review_digest.as_bytes().to_vec(),
        value.receipt.capture_digest.as_bytes().to_vec(),
        value.receipt.submission_digest.as_bytes().to_vec(),
    ]
}

pub fn stored_evidence(db: &mut Fixture, value: &DeadlineDetail) -> Vec<Vec<u8>> {
    let row = db
        .admin
        .query_one(
            "SELECT review_canonical,capture_canonical,submission_canonical,
            review_digest,capture_digest,submission_digest
         FROM case_deadline_revisions WHERE deadline_id=$1 AND revision=$2",
            &[&value.id.as_uuid(), &i64::from(value.revision.get())],
        )
        .unwrap();
    (0..6).map(|index| row.get(index)).collect()
}

pub fn revision_row(db: &mut Fixture, value: &DeadlineDetail) -> serde_json::Value {
    db.admin.query_one(
        "SELECT to_jsonb(r) FROM case_deadline_revisions r WHERE deadline_id=$1 AND revision=$2",
        &[&value.id.as_uuid(), &i64::from(value.revision.get())],
    ).unwrap().get(0)
}

pub fn users(db: &mut Fixture) -> (i64, serde_json::Value) {
    let row = db
        .admin
        .query_one(
            "SELECT count(*),coalesce(jsonb_agg(jsonb_build_object(
            'id',id,'email',email,'role',role,'active',active) ORDER BY id),'[]'::jsonb)
         FROM users",
            &[],
        )
        .unwrap();
    (row.get(0), row.get(1))
}

pub fn assert_readback(db: &Fixture, store: &dyn DeadlineStore, revisions: &[DeadlineDetail]) {
    let latest = revisions.last().unwrap();
    for expected in revisions {
        let actual = store
            .get(
                db.owner,
                db.case,
                expected.id,
                Some(expected.revision),
                db.at,
            )
            .unwrap();
        assert_eq!(&actual, expected);
        assert_eq!(canonical_evidence(&actual), canonical_evidence(expected));
        deadline_receipt_matches(&RingSha256Hasher, &actual).unwrap();
    }
    assert_eq!(
        store
            .get(db.owner, db.case, latest.id, None, db.at)
            .unwrap(),
        *latest
    );
    let history = store
        .history(db.owner, db.case, latest.id, history_query(20, None), db.at)
        .unwrap();
    assert!(!history.has_more);
    assert!(history.next_before_revision.is_none());
    assert_eq!(
        history.revisions,
        revisions
            .iter()
            .rev()
            .map(|value| DeadlineHistoryEntry::from_detail(&RingSha256Hasher, value).unwrap())
            .collect::<Vec<_>>()
    );
    for entry in history.revisions {
        deadline_history_receipt_matches(&RingSha256Hasher, &entry).unwrap();
    }
}
