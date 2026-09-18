#![allow(dead_code)]
#[path = "../deadline_evaluation_support/mod.rs"]
pub mod evaluation;
use application::{deadline_profiles::*, deadlines::*};
use domain::{
    identity::{Role, UserId},
    procedural_facts::FactLabel,
};
use evaluation::{inputs, *};
use uuid::Uuid;

pub fn fixture() -> (DeadlineCommand, DeadlinePreparation) {
    let (profile, input, material) = evaluation::fixture();
    let definition = DeadlineProfileDefinition::new(profile).unwrap();
    let profile_id = DeadlineProfileId::from_uuid(Uuid::from_u128(50));
    let profile_command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::from_uuid(Uuid::from_u128(51)),
        profile_id,
        change: DeadlineProfileChange::Publish {
            definition: definition.clone(),
        },
    };
    let definition_digest =
        deadline_profile_definition_digest(inputs::hasher().as_ref(), &definition);
    let profile = DeadlineProfileDetail {
        id: profile_id,
        revision: DeadlineProfileRevision::initial(),
        definition,
        definition_digest,
        algorithm: DeadlineProfileAlgorithm::V1,
        status: DeadlineProfileStatus::Published,
        reason: None,
        receipt: DeadlineProfileReceipt {
            operation_id: profile_command.operation_id,
            action: DeadlineProfileAction::Publish,
            expected_revision: 0,
            submission_digest: deadline_profile_submission_digest(
                inputs::hasher().as_ref(),
                inputs::actor(),
                &profile_command,
                DeadlineProfileAlgorithm::V1,
                definition_digest,
            ),
        },
        recorded_at: crate::case_support::instant(),
        recorded_by: DeadlineProfileActorSnapshot {
            id: inputs::actor(),
            email: "owner@example.com".into(),
        },
    };
    let id = DeadlineId::from_uuid(Uuid::from_u128(100));
    let definition = DeadlineDefinition {
        title: FactLabel::new("Declared response period").unwrap(),
        profile: DeadlineProfileRef {
            id: profile.id,
            revision: profile.revision,
        },
        input,
        responsible: inputs::actor(),
    };
    let command = DeadlineCommand {
        operation_id: DeadlineOperationId::from_uuid(Uuid::from_u128(101)),
        deadline_id: id,
        change: DeadlineChange::Register { definition },
    };
    let preparation = DeadlinePreparation {
        case_id: inputs::case_id(),
        deadline_id: id,
        administration: material.administration.clone(),
        base: None,
        resolved: Some(DeadlineResolvedInputs {
            profile: profile.clone(),
            profile_head: profile,
            material,
        }),
        responsible: Some(DeadlineResponsibleSnapshot {
            id: inputs::actor(),
            email: "owner@example.com".into(),
            role: Role::Owner,
        }),
    };
    (command, preparation)
}
pub fn prepare(
    command: DeadlineCommand,
    preparation: DeadlinePreparation,
) -> Result<PreparedDeadlineChange, application::ApplicationError> {
    prepare_deadline_change(
        inputs::hasher().as_ref(),
        inputs::actor(),
        inputs::case_id(),
        command,
        preparation,
    )
}
pub fn detail(prepared: &PreparedDeadlineChange) -> DeadlineDetail {
    DeadlineDetail {
        id: prepared.command().deadline_id,
        case_id: inputs::case_id(),
        revision: prepared.command().result_revision().unwrap(),
        definition: prepared.definition().clone(),
        calculation: prepared.calculation().clone(),
        responsible: prepared.responsible().clone(),
        attention: prepared.attention().clone(),
        status: prepared.status(),
        reason: prepared.command().reason().cloned(),
        receipt: DeadlineReceipt {
            operation_id: prepared.command().operation_id,
            action: prepared.command().action(),
            expected_revision: prepared.command().expected_revision(),
            review_digest: prepared.review_digest(),
            capture_digest: prepared.capture_digest(),
            submission_digest: prepared.submission_digest(),
        },
        recorded_at: crate::case_support::instant(),
        recorded_by: DeadlineActorSnapshot {
            id: inputs::actor(),
            email: "owner@example.com".into(),
        },
    }
}
pub fn followup(
    base: &DeadlineDetail,
    change: DeadlineChange,
) -> (DeadlineCommand, DeadlinePreparation) {
    (
        DeadlineCommand {
            operation_id: DeadlineOperationId::new(),
            deadline_id: base.id,
            change,
        },
        DeadlinePreparation {
            case_id: base.case_id,
            deadline_id: base.id,
            administration: base.calculation.material.administration.clone(),
            base: Some(base.clone()),
            resolved: None,
            responsible: None,
        },
    )
}
pub fn attention() -> DeadlineAttention {
    DeadlineAttention::Recorded {
        occurred_at: inputs::date("2026-01-09"),
        statement: text("Operator declares a filing; validity is not inferred"),
        locator: label("Filing receipt page 2"),
    }
}
pub fn owner() -> UserId {
    inputs::actor()
}
