use super::metadata;
use application::{deadline_reevaluation::*, deadlines::*};
use domain::{cases::CaseId, crypto::Sha256Digest, identity::UserId, procedural_facts::FactText};

fn human() -> DeadlineActorSnapshot {
    DeadlineActorSnapshot::User {
        id: UserId::new(),
        email: "owner@example.test".into(),
    }
}
fn technical() -> DeadlineActorSnapshot {
    DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    }
}
fn receipt(action: DeadlineAction) -> DeadlineReceipt {
    let digest = Sha256Digest::from_bytes(&[7; 32]).unwrap();
    let registration = action == DeadlineAction::Register;
    DeadlineReceipt {
        operation_id: DeadlineOperationId::new(),
        action,
        expected_revision: if registration { 0 } else { 1 },
        review_digest: digest,
        capture_digest: digest,
        submission_digest: digest,
        version: DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
            observations_digest: digest,
            predecessor: (!registration).then_some(PredecessorReceipt {
                submission_digest: digest,
                capture_digest: digest,
            }),
            cause: (action == DeadlineAction::Reevaluate).then_some(
                TechnicalCause::LegacyBootstrap {
                    job_id: uuid::Uuid::new_v4(),
                    policy_version: 1,
                },
            ),
        }),
    }
}
fn valid(value: &DeadlineReceipt, author: &DeadlineActorSnapshot) -> bool {
    let reason = (value.action != DeadlineAction::Register)
        .then(|| FactText::new("Declared reason").unwrap());
    metadata::shape(
        value,
        DeadlineRevision::new(value.expected_revision + 1).unwrap(),
        if value.action == DeadlineAction::Retire {
            DeadlineStatus::Retired
        } else {
            DeadlineStatus::Active
        },
        reason.as_ref(),
        author,
        CaseId::new(),
    )
    .is_ok()
}

#[test]
fn tracked_receipts_accept_all_human_actions_and_technical_reevaluation() {
    for action in [
        DeadlineAction::Register,
        DeadlineAction::Correct,
        DeadlineAction::SetAttention,
        DeadlineAction::Retire,
    ] {
        assert!(valid(&receipt(action), &human()), "{action:?}");
    }
    assert!(valid(&receipt(DeadlineAction::Reevaluate), &technical()));
}

#[test]
fn author_and_cause_must_match_the_receipt_action() {
    for action in [
        DeadlineAction::Register,
        DeadlineAction::Correct,
        DeadlineAction::SetAttention,
        DeadlineAction::Retire,
    ] {
        let mut value = receipt(action);
        assert!(!valid(&value, &technical()));
        let DeadlineReceiptVersion::Tracked(metadata) = &mut value.version else {
            unreachable!()
        };
        metadata.cause = Some(TechnicalCause::LegacyBootstrap {
            job_id: uuid::Uuid::new_v4(),
            policy_version: 1,
        });
        assert!(!valid(&value, &human()));
    }
    let mut value = receipt(DeadlineAction::Reevaluate);
    assert!(!valid(&value, &human()));
    let DeadlineReceiptVersion::Tracked(metadata) = &mut value.version else {
        unreachable!()
    };
    metadata.cause = None;
    assert!(!valid(&value, &technical()));
}

#[test]
fn predecessor_presence_matches_registration_or_successor() {
    for action in [
        DeadlineAction::Register,
        DeadlineAction::Correct,
        DeadlineAction::SetAttention,
        DeadlineAction::Retire,
        DeadlineAction::Reevaluate,
    ] {
        let mut value = receipt(action);
        let DeadlineReceiptVersion::Tracked(metadata) = &mut value.version else {
            unreachable!()
        };
        metadata.predecessor = if action == DeadlineAction::Register {
            Some(PredecessorReceipt {
                submission_digest: value.submission_digest,
                capture_digest: value.capture_digest,
            })
        } else {
            None
        };
        assert!(!valid(
            &value,
            &if action == DeadlineAction::Reevaluate {
                technical()
            } else {
                human()
            }
        ));
    }
}

#[test]
fn legacy_receipts_require_human_authorship_and_human_actions() {
    for action in [
        DeadlineAction::Register,
        DeadlineAction::Correct,
        DeadlineAction::SetAttention,
        DeadlineAction::Retire,
        DeadlineAction::Reevaluate,
    ] {
        let mut value = receipt(action);
        value.version = DeadlineReceiptVersion::Legacy;
        assert!(!valid(&value, &technical()));
        assert_eq!(
            valid(&value, &human()),
            action != DeadlineAction::Reevaluate
        );
    }
}

#[test]
fn receipt_action_state_and_reason_are_checked_for_both_versions() {
    for version in [
        DeadlineReceiptVersion::Legacy,
        receipt(DeadlineAction::Correct).version,
    ] {
        let mut value = receipt(DeadlineAction::Correct);
        value.version = version;
        for (revision, status, reason) in [
            (
                3,
                DeadlineStatus::Active,
                Some(FactText::new("Reason").unwrap()),
            ),
            (
                2,
                DeadlineStatus::Retired,
                Some(FactText::new("Reason").unwrap()),
            ),
            (2, DeadlineStatus::Active, None),
        ] {
            assert!(metadata::shape(
                &value,
                DeadlineRevision::new(revision).unwrap(),
                status,
                reason.as_ref(),
                &human(),
                CaseId::new()
            )
            .is_err());
        }
    }
}
