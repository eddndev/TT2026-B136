mod deadline_http_support;
use application::{deadline_reevaluation::*, deadline_tracking::*, deadlines::*};
use deadline_http_support::*;
use domain::procedural_facts::FactText;
use serde_json::json;
use std::sync::Arc;

fn workflow(detail: DeadlineDetail) -> Arc<Workflow> {
    let workflow = Arc::new(Workflow {
        return_draft: true,
        ..Default::default()
    });
    *workflow.response.lock().unwrap() = Some(detail);
    workflow
}
async fn exact(detail: DeadlineDetail) -> (u16, serde_json::Value) {
    let path = format!("{BASE}/{ID}/revisions/{}", detail.revision.get());
    request(workflow(detail), "GET", &path, Some("owner"), None, &[]).await
}

#[tokio::test]
async fn draft_echoes_explicit_policies_and_complete_human_tracking_metadata() {
    let value = command();
    let (status, draft) = prepare(workflow(records::fixture()), value.clone()).await;
    assert_eq!(status, 200, "{draft}");
    assert_eq!(draft["command"], value);
    assert_eq!(draft["actor_id"], ACTOR);
    assert_eq!(
        draft["author"],
        json!({"kind":"user","id":ACTOR,"email":"owner@example.com"})
    );
    assert_eq!(draft["tracking"]["policies"], value["change"]["tracking"]);
    assert_eq!(
        draft["tracking"]["review"],
        json!({"state":"accepted","reasons":[]})
    );
    assert_eq!(
        draft["receipt_version"],
        json!({"kind":"v2","observations_digest":digest().to_hex(),"predecessor":null,"cause":null})
    );
    assert!(draft.get("operational").is_none());
}

#[tokio::test]
async fn exact_legacy_and_tracked_records_preserve_calculation_without_claiming_freshness() {
    let mut calculation = None;
    for (detail, kind) in [
        (records::fixture(), "v1"),
        (records::tracked_fixture(), "v2"),
    ] {
        let (status, row) = exact(detail).await;
        assert_eq!(status, 200, "{row}");
        assert_eq!(row["receipt"]["version"]["kind"], kind);
        assert_eq!(row["recorded_by"]["kind"], "user");
        assert_eq!(row["tracking"].is_null(), kind == "v1");
        assert_eq!(
            row["operational"],
            json!({"freshness":"not_checked","checked_at":null,"changed_dependencies":[],"due_at":null})
        );
        if let Some(previous) = &calculation {
            assert_eq!(&row["calculation"], previous);
        }
        calculation = Some(row["calculation"].clone());
    }
}

fn technical(cause: TechnicalCause) -> DeadlineDetail {
    let mut detail = records::tracked_fixture();
    detail.revision = DeadlineRevision::new(2).unwrap();
    detail.receipt.action = DeadlineAction::Reevaluate;
    detail.receipt.expected_revision = 1;
    detail.reason = Some(FactText::new("Source requires human review").unwrap());
    detail.recorded_by = DeadlineActorSnapshot::Technical {
        service: TechnicalService::DeadlineReevaluator,
        policy_version: 1,
    };
    let DeadlineReceiptVersion::Tracked(metadata) = &mut detail.receipt.version else {
        unreachable!()
    };
    metadata.predecessor = Some(PredecessorReceipt {
        submission_digest: digest(),
        capture_digest: digest(),
    });
    metadata.cause = Some(cause);
    let tracking = detail.tracking.as_mut().unwrap();
    tracking.review = if matches!(cause, TechnicalCause::LegacyBootstrap { .. }) {
        tracking.policies = TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        };
        TrackingReview::new(
            DeadlineReviewState::Pending,
            [TrackingDependency::Profile, TrackingDependency::Source]
                .map(|dependency| TrackingReviewRequirement {
                    dependency,
                    reason: TrackingReviewReason::PolicyUndetermined,
                })
                .to_vec(),
        )
        .unwrap()
    } else {
        TrackingReview::new(
            DeadlineReviewState::Pending,
            vec![TrackingReviewRequirement {
                dependency: TrackingDependency::Source,
                reason: TrackingReviewReason::SourceChanged,
            }],
        )
        .unwrap()
    };
    detail
}

#[tokio::test]
async fn technical_detail_preserves_both_cause_variants_without_a_fabricated_user() {
    let causes = [
        TechnicalCause::LegacyBootstrap {
            job_id: uuid::Uuid::nil(),
            policy_version: 1,
        },
        TechnicalCause::SourceEvent {
            job_id: uuid::Uuid::nil(),
            event: SourceEventReference {
                sequence: 9_007_199_254_740_993,
                family: DependencyFamily::Resolution,
                source_id: uuid::Uuid::parse_str(SOURCE).unwrap(),
                revision: 1,
                case_id: Some(case()),
                hearing_id: None,
                operation_id: uuid::Uuid::nil(),
            },
        },
    ];
    for cause in causes {
        let (status, row) = exact(technical(cause)).await;
        assert_eq!(status, 200, "{row}");
        assert_eq!(
            row["recorded_by"],
            json!({"kind":"technical","service":"deadline_reevaluator","policy_version":1})
        );
        assert_eq!(row["responsible"]["email"], "owner@example.com");
        assert_eq!(row["tracking"]["review"]["state"], "pending");
        assert!(row["operational"]["due_at"].is_null());
        if matches!(cause, TechnicalCause::SourceEvent { .. }) {
            assert_eq!(
                row["receipt"]["version"]["cause"]["event"]["sequence"],
                "9007199254740993"
            );
        } else {
            assert_eq!(
                row["receipt"]["version"]["cause"]["kind"],
                "legacy_bootstrap"
            );
        }
    }
}

#[tokio::test]
async fn malformed_version_author_tracking_combinations_never_fall_back_to_legacy() {
    for mutation in 0..8 {
        let mut detail = records::tracked_fixture();
        match mutation {
            0 => detail.tracking = None,
            1 => detail.receipt.version = DeadlineReceiptVersion::Legacy,
            2 => {
                detail.recorded_by = DeadlineActorSnapshot::Technical {
                    service: TechnicalService::DeadlineReevaluator,
                    policy_version: 1,
                }
            }
            3 => {
                detail.tracking.as_mut().unwrap().observations.case_id =
                    domain::cases::CaseId::new()
            }
            4 => {
                detail.tracking.as_mut().unwrap().review =
                    TrackingReview::new(DeadlineReviewState::LegacyUndeclared, vec![]).unwrap()
            }
            5 => detail.tracking.as_mut().unwrap().policies.profile = TrackingPolicy::Undetermined,
            6 => {
                let DeadlineReceiptVersion::Tracked(metadata) = &mut detail.receipt.version else {
                    unreachable!()
                };
                metadata.predecessor = Some(PredecessorReceipt {
                    submission_digest: digest(),
                    capture_digest: digest(),
                });
            }
            _ => {
                let DeadlineReceiptVersion::Tracked(metadata) = &mut detail.receipt.version else {
                    unreachable!()
                };
                metadata.cause = Some(TechnicalCause::LegacyBootstrap {
                    job_id: uuid::Uuid::nil(),
                    policy_version: 1,
                });
            }
        }
        let (status, body) = exact(detail).await;
        assert_eq!(status, 500, "mutation {mutation}: {body}");
    }
}

#[tokio::test]
async fn human_confirmation_requires_v2_and_the_requested_policies() {
    for mutation in 0..3 {
        let mut detail = records::tracked_fixture();
        if mutation == 0 {
            detail = records::fixture();
        }
        if mutation == 1 {
            detail.tracking.as_mut().unwrap().policies.source = TrackingPolicy::Fixed;
        }
        if mutation == 2 {
            detail.recorded_by = DeadlineActorSnapshot::Technical {
                service: TechnicalService::DeadlineReevaluator,
                policy_version: 1,
            };
        }
        let (status, body) = request(
            workflow(detail),
            "POST",
            BASE,
            Some("owner"),
            Some(
                json!({"command":command(),"expected_submission_digest":digest().to_hex()})
                    .to_string(),
            ),
            &["application/json"],
        )
        .await;
        assert_eq!(status, 500, "mutation {mutation}: {body}");
    }
}

#[tokio::test]
async fn current_detail_uses_the_checked_projection_while_exact_history_stays_unchecked() {
    use application::{
        deadline_currentness::evaluate_deadline_currentness,
        deadline_technical::DeadlineReevaluationInputs,
    };
    let base = records::tracked_fixture();
    let inputs = DeadlineReevaluationInputs {
        profile_head: base.calculation.profile.clone(),
        material: base.calculation.material.clone(),
        notification_parent_head: None,
    };
    let current =
        evaluate_deadline_currentness(&records::Hasher, &base, Some(&inputs), records::instant())
            .unwrap();
    let flow = workflow(base);
    *flow.current_response.lock().unwrap() = Some(current);
    let (status, value) = request(
        flow.clone(),
        "GET",
        &format!("{BASE}/{ID}"),
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{value}");
    assert_eq!(value["operational"]["freshness"], "current");
    assert_eq!(
        value["operational"]["checked_at"]["unix_seconds"],
        records::instant().unix_timestamp()
    );
    let (status, history) = request(
        flow,
        "GET",
        &format!("{BASE}/{ID}/revisions/1"),
        Some("owner"),
        None,
        &[],
    )
    .await;
    assert_eq!(status, 200, "{history}");
    assert_eq!(history["operational"]["freshness"], "not_checked");
    assert_eq!(history["tracking"], value["tracking"]);
    assert_eq!(history["calculation"], value["calculation"]);
}
