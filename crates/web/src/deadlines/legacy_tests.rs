use super::{metadata, pages};
use application::deadlines::*;
use domain::{crypto::Sha256Digest, identity::UserId};
use serde_json::json;
use uuid::Uuid;

#[test]
fn legacy_http_actor_is_tagged_without_replacing_the_captured_identity() {
    let author = DeadlineActorSnapshot::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "owner@example.test".into(),
    };
    assert_eq!(
        metadata::actor(&author).unwrap(),
        json!({
            "kind":"user", "id": Uuid::nil(), "email": "owner@example.test"
        })
    );
}

#[test]
fn legacy_http_receipt_rejects_a_technical_action() {
    let receipt = DeadlineReceipt {
        operation_id: DeadlineOperationId::from_uuid(Uuid::nil()),
        action: DeadlineAction::Reevaluate,
        expected_revision: 1,
        review_digest: Sha256Digest::from_bytes(&[0; 32]).unwrap(),
        capture_digest: Sha256Digest::from_bytes(&[0; 32]).unwrap(),
        submission_digest: Sha256Digest::from_bytes(&[0; 32]).unwrap(),
        version: DeadlineReceiptVersion::Legacy,
    };
    assert!(metadata::shape(
        &receipt,
        DeadlineRevision::new(2).unwrap(),
        DeadlineStatus::Active,
        None,
        &human(),
        case(),
    )
    .is_err());
}

#[test]
fn tracked_http_receipt_accepts_coherent_human_metadata() {
    let digest = Sha256Digest::from_bytes(&[0; 32]).unwrap();
    let receipt = DeadlineReceipt {
        operation_id: DeadlineOperationId::from_uuid(Uuid::nil()),
        action: DeadlineAction::Register,
        expected_revision: 0,
        review_digest: digest,
        capture_digest: digest,
        submission_digest: digest,
        version: DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
            observations_digest: digest,
            predecessor: None,
            cause: None,
        }),
    };
    assert!(metadata::shape(
        &receipt,
        DeadlineRevision::initial(),
        DeadlineStatus::Active,
        None,
        &human(),
        case(),
    )
    .is_ok());
}

#[test]
fn legacy_http_list_rejects_unprojected_review_states() {
    use super::tracking_response_test_support::records;
    use application::{
        deadline_currentness::DeadlineCurrent, deadline_tracking::DeadlineReviewState,
    };
    let detail = records::fixture();
    let current = DeadlineCurrent::historical(&records::Hasher, &detail).unwrap();
    let case = detail.case_id;
    for review_state in [DeadlineReviewState::Accepted, DeadlineReviewState::Pending] {
        let mut row = DeadlineOverview::from(&current);
        row.review_state = review_state;
        let page = DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        };
        let query = DeadlineQuery::new(20, None, DeadlineStatusFilter::Active).unwrap();
        assert!(pages::page(page, case, &query).is_err());
    }
}

fn human() -> DeadlineActorSnapshot {
    DeadlineActorSnapshot::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "owner@example.test".into(),
    }
}
fn case() -> domain::cases::CaseId {
    domain::cases::CaseId::from_uuid(Uuid::nil())
}
