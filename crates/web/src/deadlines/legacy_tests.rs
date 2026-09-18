use super::{metadata, pages};
use application::{deadline_reevaluation::TechnicalService, deadlines::*};
use domain::{crypto::Sha256Digest, identity::UserId};
use serde_json::json;
use uuid::Uuid;

#[test]
fn legacy_http_actor_keeps_the_existing_human_json() {
    let author = DeadlineActorSnapshot::User {
        id: UserId::from_uuid(Uuid::nil()),
        email: "owner@example.test".into(),
    };
    assert_eq!(
        metadata::actor(&author).unwrap(),
        json!({
            "id": Uuid::nil(), "email": "owner@example.test"
        })
    );
}

#[test]
fn legacy_http_actor_rejects_technical_authorship() {
    let author = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    assert!(metadata::actor(&author).is_err());
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
        None
    )
    .is_err());
}

#[test]
fn legacy_http_receipt_rejects_tracked_metadata_even_for_a_human_action() {
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
    )
    .is_err());
}

#[test]
fn legacy_http_list_rejects_unprojected_review_states() {
    use application::deadline_tracking::DeadlineReviewState;
    use domain::{cases::CaseId, identity::Role, procedural_facts::FactLabel};

    let case = CaseId::from_uuid(Uuid::nil());
    for review_state in [DeadlineReviewState::Accepted, DeadlineReviewState::Pending] {
        let row = DeadlineOverview {
            id: DeadlineId::from_uuid(Uuid::nil()),
            case_id: case,
            revision: DeadlineRevision::initial(),
            title: FactLabel::new("Declared period").unwrap(),
            status: DeadlineStatus::Active,
            responsible: DeadlineResponsibleSnapshot {
                id: UserId::from_uuid(Uuid::nil()),
                email: "owner@example.test".into(),
                role: Role::Owner,
            },
            attention_recorded: false,
            due_at: None,
            blocked: true,
            review_state,
        };
        let page = DeadlinePage {
            deadlines: vec![row],
            has_more: false,
            next_after_id: None,
        };
        let query = DeadlineQuery::new(20, None, DeadlineStatusFilter::Active).unwrap();
        assert!(pages::page(page, case, &query).is_err());
    }
}
