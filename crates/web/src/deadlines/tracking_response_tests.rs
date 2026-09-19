use super::{pages, request, response, tracking_response_test_support::*};
use application::{deadline_reevaluation::*, deadline_tracking::*, deadlines::*};
use domain::{crypto::Sha256Digest, identity::UserId, procedural_facts::FactText};
use serde_json::json;

fn human() -> DeadlineHumanCommand {
    serde_json::from_value::<request::Command>(command())
        .unwrap()
        .validate()
        .unwrap()
}

#[test]
fn draft_rejects_inconsistent_author_version_policy_and_observation_metadata() {
    let expected = human();
    for mutation in 0..9 {
        let mut draft = records::draft(expected.clone(), &records::fixture());
        match mutation {
            0 => {
                draft.author = DeadlineActorSnapshot::Technical {
                    service: TechnicalService::DeadlineReevaluator,
                    policy_version: 1,
                }
            }
            1 => draft.actor = UserId::new(),
            2 => draft.receipt_version = DeadlineReceiptVersion::Legacy,
            3 => draft.tracking.policies.source = TrackingPolicy::Fixed,
            4 => draft.tracking.observations.case_id = domain::cases::CaseId::new(),
            5 => draft.tracking.policies.profile = TrackingPolicy::Undetermined,
            6 => {
                draft.tracking.review = TrackingReview::new(
                    DeadlineReviewState::Pending,
                    vec![TrackingReviewRequirement {
                        dependency: TrackingDependency::Source,
                        reason: TrackingReviewReason::SourceChanged,
                    }],
                )
                .unwrap()
            }
            7 => {
                let DeadlineReceiptVersion::Tracked(metadata) = &mut draft.receipt_version else {
                    unreachable!()
                };
                metadata.cause = Some(TechnicalCause::LegacyBootstrap {
                    job_id: uuid::Uuid::nil(),
                    policy_version: 1,
                });
            }
            _ => {
                let DeadlineReceiptVersion::Tracked(metadata) = &mut draft.receipt_version else {
                    unreachable!()
                };
                metadata.predecessor = Some(PredecessorReceipt {
                    submission_digest: digest(),
                    capture_digest: digest(),
                });
            }
        }
        assert!(
            response::draft(draft, case(), &expected).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn retained_tracking_is_displayed_but_never_added_to_attention_or_retirement_commands() {
    for action in ["set_attention", "retire"] {
        let mut input = command();
        input["change"] = json!({"action":action,"expected_revision":1,"reason":"Declared action"});
        if action == "set_attention" {
            input["change"]["attention"] = json!({"status":"pending"});
        }
        let expected = serde_json::from_value::<request::Command>(input.clone())
            .unwrap()
            .validate()
            .unwrap();
        for base in [records::fixture(), records::tracked_fixture()] {
            let draft = records::draft(expected.clone(), &base);
            let result = response::draft(draft, case(), &expected).unwrap();
            assert_eq!(result["command"], input);
            assert!(result["command"]["change"].get("tracking").is_none());
            assert!(result["tracking"].is_object());
            assert_eq!(
                result["receipt_version"]["predecessor"]["capture_digest"],
                digest().to_hex()
            );
            assert_eq!(
                result["tracking"]["review"]["state"],
                if base.tracking.is_some() {
                    "accepted"
                } else {
                    "legacy_undeclared"
                }
            );
        }
    }
}

fn history(base: DeadlineDetail) -> DeadlineHistoryPage {
    let command = DeadlineHumanCommand::new(
        DeadlineCommand {
            operation_id: DeadlineOperationId::new(),
            deadline_id: base.id,
            change: DeadlineChange::Correct {
                expected_revision: base.revision,
                definition: base.definition.clone(),
                reason: FactText::new("Declared correction").unwrap(),
            },
        },
        Some(records::tracking_policies()),
    )
    .unwrap();
    let next = records::tracked_change(command, &base);
    DeadlineHistoryPage {
        revisions: vec![
            DeadlineHistoryEntry::from_detail(&records::Hasher, &next).unwrap(),
            DeadlineHistoryEntry::from_detail(&records::Hasher, &base).unwrap(),
        ],
        has_more: false,
        next_before_revision: None,
    }
}

#[test]
fn mixed_and_tracked_history_preserve_exact_predecessor_commitments() {
    for base in [records::fixture(), records::tracked_fixture()] {
        let id = base.id;
        let page = history(base);
        let result = pages::history(
            page.clone(),
            case(),
            id,
            DeadlineHistoryQuery::new(20, None).unwrap(),
        )
        .unwrap();
        assert_eq!(result["revisions"][0]["receipt"]["version"]["kind"], "v2");
        for mutation in 0..3 {
            let mut invalid = page.clone();
            if mutation == 2 {
                invalid.revisions[0].receipt.version = DeadlineReceiptVersion::Legacy;
                invalid.revisions[1].receipt.version =
                    DeadlineReceiptVersion::Tracked(DeadlineTrackedReceipt {
                        observations_digest: digest(),
                        predecessor: None,
                        cause: None,
                    });
            } else {
                let DeadlineReceiptVersion::Tracked(metadata) =
                    &mut invalid.revisions[0].receipt.version
                else {
                    unreachable!()
                };
                let previous = metadata.predecessor.as_mut().unwrap();
                if mutation == 0 {
                    previous.submission_digest = Sha256Digest::from_array([8; 32]);
                } else {
                    previous.capture_digest = Sha256Digest::from_array([8; 32]);
                }
            }
            assert!(
                pages::history(
                    invalid,
                    case(),
                    id,
                    DeadlineHistoryQuery::new(20, None).unwrap()
                )
                .is_err(),
                "mutation {mutation}"
            );
        }
    }
}
