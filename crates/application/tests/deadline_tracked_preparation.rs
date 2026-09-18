#[allow(dead_code)]
mod case_support;
#[allow(dead_code)]
#[path = "support/document_workflow.rs"]
mod crypto;
mod deadline_observation_support;
mod deadline_support;
mod deadline_tracked_support;
use application::{
    deadline_inputs::DeadlineSourceDetail, deadline_reevaluation::*, deadline_tracking::*,
    deadlines::*,
};
use deadline_support::{
    evaluation::{inputs, text},
    *,
};
use deadline_tracked_support::{legacy, pending};

fn policies() -> TrackingPolicies {
    TrackingPolicies {
        profile: TrackingPolicy::Follow,
        source: TrackingPolicy::Follow,
        calendar: TrackingPolicy::Undetermined,
    }
}
fn author() -> DeadlineActorSnapshot {
    DeadlineActorSnapshot::User {
        id: inputs::actor(),
        email: "owner@example.com".into(),
    }
}
fn tracked(
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
    policies: Option<TrackingPolicies>,
) -> Result<PreparedDeadlineChange, application::ApplicationError> {
    prepare_tracked_deadline_change(
        inputs::hasher().as_ref(),
        author(),
        inputs::case_id(),
        command,
        preparation,
        policies,
        None,
    )
}
fn record(value: &PreparedDeadlineChange) -> DeadlineDetail {
    let mut result = detail(value);
    result.tracking = value.tracking().cloned();
    result.receipt = value.receipt();
    result.recorded_by = value.tracked_author().unwrap().clone();
    deadline_receipt_matches(inputs::hasher().as_ref(), &result).unwrap();
    result
}
fn registration() -> DeadlineDetail {
    let (command, preparation) = fixture();
    record(&tracked(command, preparation, Some(policies())).unwrap())
}
#[test]
fn registration_captures_verified_observations_and_explicit_human_acceptance() {
    let value = registration();
    assert_eq!(value.review_state(), DeadlineReviewState::Accepted);
    assert!(value.operational_due_at().is_some());
    let tracking = value.tracking.as_ref().unwrap();
    assert_eq!(tracking.policies, policies());
    assert_eq!(tracking.observations.entries.len(), 2);
    assert_eq!(
        tracking.observations.entries[0].submission_digest,
        value.calculation.profile.receipt.submission_digest
    );
    let DeadlineReceiptVersion::Tracked(receipt) = &value.receipt.version else {
        panic!()
    };
    assert_eq!(receipt.predecessor, None);
    assert_eq!(receipt.cause, None);
    assert_eq!(value.recorded_by, author());
}
#[test]
fn registration_cannot_infer_policies_or_accept_a_policy_for_an_absent_calendar() {
    for policies in [
        None,
        Some(TrackingPolicies {
            source: TrackingPolicy::Undetermined,
            ..policies()
        }),
        Some(TrackingPolicies {
            calendar: TrackingPolicy::Follow,
            ..policies()
        }),
    ] {
        let (command, preparation) = fixture();
        assert!(tracked(command, preparation, policies).is_err());
    }
}
#[test]
fn fixed_source_selection_can_be_accepted_but_follow_requires_the_observed_revision() {
    let (command, mut preparation) = fixture();
    preparation.resolved.as_mut().unwrap().material.source_head = Some(DeadlineSourceDetail::Fact(
        Box::new(inputs::resolution(2, false, "2026-01-10")),
    ));
    assert!(tracked(command.clone(), preparation.clone(), Some(policies())).is_err());
    let fixed = TrackingPolicies {
        source: TrackingPolicy::Fixed,
        ..policies()
    };
    let value = record(&tracked(command, preparation, Some(fixed)).unwrap());
    assert_eq!(
        value.tracking.as_ref().unwrap().observations.entries[1].revision,
        2
    );
    assert_eq!(value.calculation.result, legacy().calculation.result);
    assert!(value.operational_due_at().is_some());
}
#[test]
fn attention_and_retirement_preserve_tracking_without_accepting_new_policies() {
    let base = pending(&registration());
    for change in [
        DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: attention(),
            reason: text("Declared filing"),
        },
        DeadlineChange::Retire {
            expected_revision: base.revision,
            reason: text("Retire record"),
        },
    ] {
        let (command, preparation) = followup(&base, change);
        assert!(tracked(command.clone(), preparation.clone(), Some(policies())).is_err());
        let value = record(&tracked(command, preparation, None).unwrap());
        assert_eq!(value.tracking, base.tracking);
        assert_eq!(value.calculation, base.calculation);
        assert_eq!(value.responsible, base.responsible);
        assert_eq!(value.operational_due_at(), None);
        let DeadlineReceiptVersion::Tracked(receipt) = value.receipt.version else {
            panic!()
        };
        assert_eq!(
            receipt.predecessor.unwrap().capture_digest,
            base.receipt.capture_digest
        );
    }
}
#[test]
fn a_human_attention_on_legacy_history_does_not_invent_acceptance() {
    let base = legacy();
    let (command, preparation) = followup(
        &base,
        DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: attention(),
            reason: text("Declared filing"),
        },
    );
    let value = record(&tracked(command, preparation, None).unwrap());
    assert_eq!(value.review_state(), DeadlineReviewState::LegacyUndeclared);
    assert_eq!(
        value.tracking.unwrap().policies,
        TrackingPolicies {
            profile: TrackingPolicy::Undetermined,
            source: TrackingPolicy::Undetermined,
            calendar: TrackingPolicy::Undetermined,
        }
    );
    assert_eq!(value.calculation, base.calculation);
}
#[test]
fn an_explicit_correction_migrates_legacy_history_and_preserves_attention() {
    let initial = legacy();
    let (attention_command, attention_preparation) = followup(
        &initial,
        DeadlineChange::SetAttention {
            expected_revision: initial.revision,
            attention: attention(),
            reason: text("Declared filing"),
        },
    );
    let base = detail(&prepare(attention_command, attention_preparation).unwrap());
    let (mut command, mut preparation) = fixture();
    command.change = DeadlineChange::Correct {
        expected_revision: base.revision,
        definition: base.definition.clone(),
        reason: text("Explicit qualification"),
    };
    preparation.base = Some(base.clone());
    let value = record(&tracked(command, preparation, Some(policies())).unwrap());
    assert_eq!(value.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(value.attention, base.attention);
    assert!(value.operational_due_at().is_some());
}
#[test]
fn the_human_preparer_rejects_technical_authors() {
    let (command, preparation) = fixture();
    assert!(prepare_tracked_deadline_change(
        inputs::hasher().as_ref(),
        DeadlineActorSnapshot::Technical {
            service: TechnicalService::DeadlineReevaluator,
            policy_version: 1
        },
        inputs::case_id(),
        command,
        preparation,
        Some(policies()),
        None,
    )
    .is_err());
}
#[test]
fn the_legacy_writer_cannot_discard_an_existing_tracking_capture() {
    let base = registration();
    let (command, preparation) = followup(
        &base,
        DeadlineChange::SetAttention {
            expected_revision: base.revision,
            attention: attention(),
            reason: text("Declared filing"),
        },
    );
    assert!(prepare(command, preparation).is_err());
}

#[test]
fn an_explicit_fixed_profile_can_keep_a_published_historical_revision() {
    use application::deadline_profiles::*;
    for retired in [false, true] {
        let (command, mut preparation) = fixture();
        let head = &mut preparation.resolved.as_mut().unwrap().profile_head;
        let replacement = DeadlineProfileCommand {
            operation_id: DeadlineProfileOperationId::new(),
            profile_id: head.id,
            change: if retired {
                DeadlineProfileChange::Retire {
                    expected_revision: head.revision,
                    reason: text("Retired profile"),
                }
            } else {
                DeadlineProfileChange::Replace {
                    expected_revision: head.revision,
                    definition: head.definition.clone(),
                    reason: text("Replaced profile"),
                }
            },
        };
        head.revision = replacement.result_revision().unwrap();
        head.status = replacement.result_status();
        head.reason = replacement.reason().cloned();
        head.receipt = DeadlineProfileReceipt {
            operation_id: replacement.operation_id,
            action: replacement.action(),
            expected_revision: replacement.expected_revision(),
            submission_digest: deadline_profile_submission_digest(
                inputs::hasher().as_ref(),
                head.recorded_by.id,
                &replacement,
                head.algorithm,
                head.definition_digest,
            ),
        };
        assert!(tracked(command.clone(), preparation.clone(), Some(policies())).is_err());
        let fixed = TrackingPolicies {
            profile: TrackingPolicy::Fixed,
            ..policies()
        };
        let result = tracked(command, preparation, Some(fixed));
        if retired {
            assert!(result.is_err());
        } else {
            let value = record(&result.unwrap());
            assert_eq!(value.definition.profile.revision.get(), 1);
            assert_eq!(value.tracking.unwrap().observations.entries[0].revision, 2);
        }
    }
}
#[test]
fn equal_profile_revisions_cannot_replace_the_captured_offset() {
    let (command, mut preparation) = fixture();
    let resolved = preparation.resolved.as_mut().unwrap();
    resolved.profile_head.recorded_at = resolved
        .profile_head
        .recorded_at
        .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(resolved.profile, resolved.profile_head);
    assert!(tracked(command, preparation, Some(policies())).is_err());
}
#[test]
fn fixing_a_source_revision_does_not_implicitly_accept_its_withdrawn_head() {
    let (command, mut preparation) = fixture();
    preparation.resolved.as_mut().unwrap().material.source_head = Some(DeadlineSourceDetail::Fact(
        Box::new(inputs::resolution(2, true, "2026-01-10")),
    ));
    let fixed = TrackingPolicies {
        source: TrackingPolicy::Fixed,
        ..policies()
    };
    assert!(tracked(command, preparation, Some(fixed)).is_err());
}

#[test]
fn explicit_review_does_not_invent_a_due_for_an_unknown_source() {
    let (mut command, mut preparation) = fixture();
    let DeadlineChange::Register { definition } = &mut command.change else {
        panic!()
    };
    definition.input.selection.source =
        domain::procedural_facts::FactDeclaration::Unknown(text("Source not yet known"));
    let material = &mut preparation.resolved.as_mut().unwrap().material;
    material.source = None;
    material.source_head = None;
    let declared = TrackingPolicies {
        source: TrackingPolicy::Undetermined,
        ..policies()
    };
    let value = record(&tracked(command, preparation, Some(declared)).unwrap());
    assert_eq!(value.review_state(), DeadlineReviewState::Accepted);
    assert_eq!(value.operational_due_at(), None);
    assert_eq!(value.tracking.unwrap().observations.entries.len(), 1);
}
#[test]
fn prepared_administration_must_match_the_exact_resolved_capture_including_offset() {
    let (command, mut preparation) = fixture();
    preparation.administration =
        deadline_observation_support::administration(inputs::case_id(), false);
    preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration = preparation.administration.clone();
    let application::cases::CurrentCaseAdministration::Recorded(admin) = &mut preparation
        .resolved
        .as_mut()
        .unwrap()
        .material
        .administration
    else {
        panic!()
    };
    admin.changed_at = admin
        .changed_at
        .to_offset(time::UtcOffset::from_hms(2, 0, 0).unwrap());
    assert_eq!(
        preparation.administration,
        preparation
            .resolved
            .as_ref()
            .unwrap()
            .material
            .administration
    );
    assert!(tracked(command, preparation, Some(policies())).is_err());
}
