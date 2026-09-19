mod case_administration_support;
mod case_stage_database_support;
#[allow(dead_code)]
#[path = "../../application/tests/support/document_workflow.rs"]
mod crypto;
mod deadline_backend_support;
mod deadline_profile_database_support;
mod deadline_tracked_backend_support;
mod procedural_fact_backend_support;

use application::{
    deadline_profiles::DeadlineProfileCollection,
    deadline_reevaluation::{ObservationRole, PredecessorReceipt},
    deadline_tracking::DeadlineReviewState,
    deadlines::*,
    ApplicationError,
};
use deadline_backend_support::*;
use deadline_tracked_backend_support::*;
use domain::identity::Role;
use infrastructure::RingSha256Hasher;

#[test]
fn tracked_human_registration_roundtrips_exact_capture_receipt_and_history() {
    let Some(mut db) = Fixture::new() else { return };
    let command = setup(&db);
    let repository = store(&db);
    let before = snapshot(&mut db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
    let expected = prepared_detail(&db, &prepared);
    deadline_receipt_matches(&RingSha256Hasher, &expected).unwrap();
    assert_eq!(snapshot(&mut db), before);
    assert_eq!(expected.review_state(), DeadlineReviewState::Accepted);
    assert!(matches!(
        expected.receipt.version,
        DeadlineReceiptVersion::Tracked(_)
    ));
    let result = repository
        .commit(db.owner, prepared)
        .expect("commit must retain the explicitly prepared human V2 capture");
    assert_eq!(result, expected);
    assert_eq!(result.recorded_by.user_id(), Some(db.owner));
    assert_eq!(result.recorded_by.email(), Some("owner@example.test"));
    assert!(result.operational_due_at().is_some());
    let tracking = result.tracking.as_ref().unwrap();
    assert_eq!(tracking.policies, policies());
    assert_eq!(
        tracking
            .observations
            .entries
            .iter()
            .map(|entry| entry.role)
            .collect::<Vec<_>>(),
        vec![ObservationRole::Profile, ObservationRole::Source]
    );
    let bytes = stored_evidence(&mut db, &result);
    assert_eq!(bytes, canonical_evidence(&result));
    for (value, prefix) in bytes.iter().zip([b"DLRV2", b"DLST2", b"DLTX2"]) {
        assert_eq!(&value[..5], prefix);
    }
    assert_readback(&db, repository.as_ref(), &[result]);
}

#[test]
fn human_upgrade_preserves_v1_bytes_and_attention_until_explicit_v2_correction() {
    let Some(mut db) = Fixture::new() else { return };
    let workflow = service(&db, db.owner, Role::Owner);
    let first = persist(&workflow, db.case, setup(&db));
    assert_eq!(first.receipt.version, DeadlineReceiptVersion::Legacy);
    assert!(first.tracking.is_none());
    let original_row = revision_row(&mut db, &first);
    let original_bytes = stored_evidence(&mut db, &first);
    assert_eq!(original_bytes, canonical_evidence(&first));
    for (value, prefix) in original_bytes.iter().zip([b"DLRV1", b"DLST1", b"DLTX1"]) {
        assert_eq!(&value[..5], prefix);
    }
    let repository = store(&db);
    let command = attention(&first);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, None);
    let expected_second = prepared_detail(&db, &prepared);
    deadline_receipt_matches(&RingSha256Hasher, &expected_second).unwrap();
    let second = repository
        .commit(db.owner, prepared)
        .expect("human attention must upgrade V1 without accepting its policies");
    assert_eq!(second, expected_second);
    assert_eq!(second.id, first.id);
    assert_eq!(second.revision.get(), 2);
    assert_eq!(second.definition, first.definition);
    assert_eq!(second.calculation, first.calculation);
    assert_eq!(second.responsible, first.responsible);
    assert_eq!(second.review_state(), DeadlineReviewState::LegacyUndeclared);
    assert!(second.operational_due_at().is_none());
    assert!(matches!(
        second.attention,
        DeadlineAttention::Recorded { .. }
    ));
    let DeadlineReceiptVersion::Tracked(metadata) = &second.receipt.version else {
        panic!("V2 expected")
    };
    assert_eq!(
        metadata.predecessor,
        Some(PredecessorReceipt {
            submission_digest: first.receipt.submission_digest,
            capture_digest: first.receipt.capture_digest,
        })
    );
    assert!(metadata.cause.is_none());
    deadline_successor_matches(&RingSha256Hasher, &first, &second).unwrap();
    assert_eq!(revision_row(&mut db, &first), original_row);
    assert_eq!(stored_evidence(&mut db, &first), original_bytes);

    let command = correct(&second);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
    let expected_third = prepared_detail(&db, &prepared);
    deadline_receipt_matches(&RingSha256Hasher, &expected_third).unwrap();
    let third = repository
        .commit(db.owner, prepared)
        .expect("explicit human correction must accept current V2 dependencies");
    assert_eq!(third, expected_third);
    assert_eq!(third.id, first.id);
    assert_eq!(third.revision.get(), 3);
    assert_eq!(third.review_state(), DeadlineReviewState::Accepted);
    assert!(third.operational_due_at().is_some());
    assert_eq!(third.attention, second.attention);
    assert_eq!(third.calculation, second.calculation);
    assert_eq!(third.responsible, second.responsible);
    assert_eq!(third.tracking.as_ref().unwrap().policies, policies());
    deadline_successor_matches(&RingSha256Hasher, &second, &third).unwrap();
    assert_eq!(revision_row(&mut db, &first), original_row);
    assert_eq!(stored_evidence(&mut db, &first), original_bytes);
    assert_eq!(
        stored_evidence(&mut db, &second),
        canonical_evidence(&second)
    );
    assert_eq!(stored_evidence(&mut db, &third), canonical_evidence(&third));
    assert_readback(&db, repository.as_ref(), &[first, second, third]);
}

#[test]
fn repeated_migration_and_reopen_preserve_v2_history_without_creating_users() {
    let Some(mut db) = Fixture::new() else { return };
    let original_users = users(&mut db);
    assert_eq!(original_users.0, 1);
    let command = setup(&db);
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
    let first = repository
        .commit(db.owner, prepared)
        .expect("V2 registration must commit");
    let prepared = tracked_prepared(&db, repository.as_ref(), &attention(&first), None);
    let second = repository
        .commit(db.owner, prepared)
        .expect("V2 attention must commit");
    assert_eq!(second.review_state(), DeadlineReviewState::Accepted);
    let rows = [
        revision_row(&mut db, &first),
        revision_row(&mut db, &second),
    ];
    drop(repository);
    for _ in 0..2 {
        let before = snapshot(&mut db);
        db.migrate();
        let reopened = store(&db);
        assert_eq!(snapshot(&mut db), before);
        assert_eq!(users(&mut db), original_users);
        for (index, value) in [&first, &second].into_iter().enumerate() {
            assert_eq!(revision_row(&mut db, value), rows[index]);
            assert_eq!(stored_evidence(&mut db, value), canonical_evidence(value));
        }
        assert_readback(&db, reopened.as_ref(), &[first.clone(), second.clone()]);
        assert_eq!(users(&mut db), original_users);
    }
}

#[test]
fn another_active_owner_cannot_commit_a_human_v2_preparation() {
    let Some(mut db) = Fixture::new() else { return };
    let other = db.user("owner", false);
    let command = setup(&db);
    let repository = store(&db);
    let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
    deadline_receipt_matches(&RingSha256Hasher, &prepared_detail(&db, &prepared)).unwrap();
    let before = snapshot(&mut db);
    assert!(matches!(
        repository.commit(other, prepared),
        Err(ApplicationError::InvalidSession)
    ));
    assert_eq!(snapshot(&mut db), before);
}

#[test]
fn stale_human_v2_heads_fail_without_appending_deadline_state_or_audit() {
    let Some(mut db) = Fixture::new() else { return };
    for change_profile in [false, true] {
        let profile = profile(&db);
        let source = source(&db);
        let command = command(&db, &profile, &source);
        let repository = store(&db);
        let prepared = tracked_prepared(&db, repository.as_ref(), &command, Some(policies()));
        deadline_receipt_matches(&RingSha256Hasher, &prepared_detail(&db, &prepared)).unwrap();
        if change_profile {
            let workflow = deadline_profile_database_support::service(&db, db.owner, Role::Owner);
            deadline_profile_database_support::persist(
                &workflow,
                DeadlineProfileCollection::ForCase(db.case),
                deadline_profile_database_support::replace(&profile),
            );
        } else {
            let workflow = procedural_fact_backend_support::service(&db, db.owner, Role::Owner);
            procedural_fact_backend_support::persist(
                &workflow,
                db.case,
                procedural_fact_backend_support::correct(&source),
            );
        }
        let before = snapshot(&mut db);
        assert!(repository.commit(db.owner, prepared).is_err());
        assert_eq!(snapshot(&mut db), before);
    }
}
