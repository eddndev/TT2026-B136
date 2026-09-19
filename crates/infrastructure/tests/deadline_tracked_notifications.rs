mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod deadline_tracked_notification_support;
mod procedural_fact_backend_support;

use application::{
    deadline_inputs::DeadlineSourceDetail,
    deadline_reevaluation::{
        encode_observations, DependencyFamily, ObservationRole, ResolutionReference,
    },
    deadline_tracking::{DeadlineReviewState, TrackingPolicies, TrackingPolicy},
    deadlines::*,
    ApplicationError,
};
use deadline_backend_support as dl;
use deadline_tracked_backend_support as tracked;
use deadline_tracked_notification_support as notification;
use infrastructure::RingSha256Hasher;
use procedural_fact_backend_support as facts;

#[test]
fn notification_v2_keeps_exact_parent_and_observed_head_after_parent_advances() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (command, parent, notice) = notification::setup(&db);
    let parent_head = notification::advance_parent(&db, &parent);
    let repository = dl::store(&db);
    let prepared = notification::prepare(&db, repository.as_ref(), &command, &parent_head);
    let expected = tracked::prepared_detail(&db, &prepared);
    deadline_receipt_matches(&RingSha256Hasher, &expected).unwrap();
    let result = repository
        .commit(db.owner, prepared)
        .expect("notification V2 must persist its separate observed parent head");
    assert_eq!(result, expected);
    assert_eq!(result.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(
        result.calculation.material.source,
        Some(DeadlineSourceDetail::Fact(Box::new(notice.clone())))
    );
    assert_eq!(
        result.calculation.material.source_head,
        Some(DeadlineSourceDetail::Fact(Box::new(notice.clone())))
    );
    let observations = &result.tracking.as_ref().unwrap().observations;
    assert_eq!(
        observations
            .entries
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        vec![
            ObservationRole::Profile,
            ObservationRole::Source,
            ObservationRole::NotificationParent
        ]
    );
    let source = &observations.entries[1];
    assert_eq!(source.family, DependencyFamily::Notification);
    assert_eq!(
        source.submission_digest,
        notice.snapshot.metadata().receipt.submission_digest
    );
    assert_eq!(
        source.parent_resolution,
        Some(ResolutionReference {
            id: facts::resolution_ref(&parent).id.as_uuid(),
            revision: 1,
        })
    );
    let observed_parent = &observations.entries[2];
    assert_eq!(observed_parent.family, DependencyFamily::Resolution);
    assert_eq!(
        observed_parent.id,
        facts::resolution_ref(&parent).id.as_uuid()
    );
    assert_eq!(observed_parent.revision, 2);
    assert_eq!(observed_parent.case_id, Some(db.case));
    assert_eq!(
        observed_parent.submission_digest,
        parent_head.snapshot.metadata().receipt.submission_digest
    );
    assert!(observed_parent.parent_resolution.is_none());
    let stored: Vec<u8> = db
        .admin
        .query_one(
            "SELECT observations_canonical FROM case_deadline_revisions WHERE deadline_id=$1",
            &[&result.id.as_uuid()],
        )
        .unwrap()
        .get(0);
    assert_eq!(stored, encode_observations(observations).unwrap());
    let row_before = tracked::revision_row(&mut db, &result);
    let bytes_before = tracked::stored_evidence(&mut db, &result);
    assert_eq!(bytes_before, tracked::canonical_evidence(&result));
    let advanced = notification::advance_parent(&db, &parent_head);
    assert_eq!(advanced.snapshot.metadata().revision.get(), 3);
    let reopened = dl::store(&db);
    tracked::assert_readback(&db, reopened.as_ref(), std::slice::from_ref(&result));
    assert_eq!(tracked::revision_row(&mut db, &result), row_before);
    assert_eq!(tracked::stored_evidence(&mut db, &result), bytes_before);
}

#[test]
fn parent_advanced_after_v2_preparation_rejects_without_deadline_or_audit_write() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (command, parent, _) = notification::setup(&db);
    let repository = dl::store(&db);
    let prepared = notification::prepare(&db, repository.as_ref(), &command, &parent);
    deadline_receipt_matches(&RingSha256Hasher, &tracked::prepared_detail(&db, &prepared)).unwrap();
    notification::advance_parent(&db, &parent);
    let before = dl::snapshot(&mut db);
    assert!(matches!(
        repository.commit(db.owner, prepared),
        Err(ApplicationError::Deadline(
            DeadlineError::SubmissionMismatch
        ))
    ));
    assert_eq!(dl::snapshot(&mut db), before);
}

#[test]
fn legacy_notification_attention_upgrade_does_not_invent_an_observed_parent() {
    let Some(mut db) = dl::Fixture::new() else {
        return;
    };
    let (command, parent, notice) = notification::setup(&db);
    let first = dl::persist_legacy(&db, db.owner, command);
    assert_eq!(first.receipt.version, DeadlineReceiptVersion::Legacy);
    assert!(first.tracking.is_none());
    let original_row = tracked::revision_row(&mut db, &first);
    let original_bytes = tracked::stored_evidence(&mut db, &first);
    assert_eq!(original_bytes, tracked::canonical_evidence(&first));
    notification::advance_parent(&db, &parent);
    let repository = dl::store(&db);
    let prepared =
        tracked::tracked_prepared(&db, repository.as_ref(), &dl::attention(&first), None);
    let expected = tracked::prepared_detail(&db, &prepared);
    let second = repository
        .commit(db.owner, prepared)
        .expect("legacy notification attention must not require a fabricated parent observation");
    assert_eq!(second, expected);
    assert_eq!(second.review_state(), DeadlineReviewState::LegacyUndeclared);
    assert!(second.operational_due_at().is_none());
    assert_eq!(second.definition, first.definition);
    assert_eq!(second.calculation, first.calculation);
    assert_eq!(second.responsible, first.responsible);
    assert!(matches!(
        second.attention,
        DeadlineAttention::Recorded { .. }
    ));
    let tracking = second.tracking.as_ref().unwrap();
    assert_eq!(
        tracking.policies,
        TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        }
    );
    assert_eq!(
        tracking
            .observations
            .entries
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        vec![ObservationRole::Profile, ObservationRole::Source]
    );
    assert_eq!(
        tracking.observations.entries[1].parent_resolution,
        Some(ResolutionReference {
            id: facts::resolution_ref(&parent).id.as_uuid(),
            revision: 1,
        })
    );
    assert_eq!(
        second.calculation.material.source,
        Some(DeadlineSourceDetail::Fact(Box::new(notice)))
    );
    deadline_successor_matches(&RingSha256Hasher, &first, &second).unwrap();
    deadline_receipt_matches(&RingSha256Hasher, &second).unwrap();
    assert_eq!(tracked::revision_row(&mut db, &first), original_row);
    assert_eq!(tracked::stored_evidence(&mut db, &first), original_bytes);
    assert_eq!(
        tracked::stored_evidence(&mut db, &second),
        tracked::canonical_evidence(&second)
    );
    let reopened = dl::store(&db);
    tracked::assert_readback(&db, reopened.as_ref(), &[first, second]);
}
