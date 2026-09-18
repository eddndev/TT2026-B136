#![allow(dead_code)]
use crate::deadline_support::evaluation;
pub mod hearing;
pub mod vectors;
use application::{
    cases::*, deadline_inputs::*, deadline_observations::build_deadline_observations,
    deadline_profiles::*, deadline_reevaluation::*, procedural_facts::*, ApplicationError,
};
use domain::{
    case_administration::{CaseAdministrationValues, CaseAdministrativeStatus, CaseRevision},
    cases::{CaseId, CaseMetadata},
    crypto::Sha256Digest,
    judicial_calendars::JudicialCalendarClassification,
};
pub use evaluation::inputs;
use uuid::Uuid;

pub fn fixture() -> (DeadlineProfileDetail, DeadlineInputMaterial) {
    let (definition, _, material) = evaluation::fixture();
    let definition = DeadlineProfileDefinition::new(definition).unwrap();
    let definition_digest =
        deadline_profile_definition_digest(inputs::hasher().as_ref(), &definition);
    let profile_id = DeadlineProfileId::from_uuid(Uuid::from_u128(50));
    let command = DeadlineProfileCommand {
        operation_id: DeadlineProfileOperationId::from_uuid(Uuid::from_u128(51)),
        profile_id,
        change: DeadlineProfileChange::Publish {
            definition: definition.clone(),
        },
    };
    (
        DeadlineProfileDetail {
            id: profile_id,
            revision: DeadlineProfileRevision::initial(),
            definition,
            definition_digest,
            algorithm: DeadlineProfileAlgorithm::V1,
            status: DeadlineProfileStatus::Published,
            reason: None,
            receipt: DeadlineProfileReceipt {
                operation_id: command.operation_id,
                action: DeadlineProfileAction::Publish,
                expected_revision: 0,
                submission_digest: deadline_profile_submission_digest(
                    inputs::hasher().as_ref(),
                    inputs::actor(),
                    &command,
                    DeadlineProfileAlgorithm::V1,
                    definition_digest,
                ),
            },
            recorded_at: crate::case_support::instant(),
            recorded_by: DeadlineProfileActorSnapshot {
                id: inputs::actor(),
                email: "owner@example.com".into(),
            },
        },
        material,
    )
}

pub fn profile_input(profile: &DeadlineProfileDetail) -> DeadlineProfileDefinitionInput {
    let v = &profile.definition;
    DeadlineProfileDefinitionInput {
        title: v.title().clone(),
        description: v.description().clone(),
        scope: v.scope().clone(),
        references: v.references().to_vec(),
        trigger: v.trigger(),
        template: v.template(),
        completion: v.completion().clone(),
        conditions: v.conditions().to_vec(),
        examples: v.examples().to_vec(),
    }
}
pub fn resign_profile(value: &mut DeadlineProfileDetail) {
    let change = match value.receipt.action {
        DeadlineProfileAction::Publish => DeadlineProfileChange::Publish {
            definition: value.definition.clone(),
        },
        DeadlineProfileAction::Replace => DeadlineProfileChange::Replace {
            expected_revision: DeadlineProfileRevision::new(value.receipt.expected_revision)
                .unwrap(),
            definition: value.definition.clone(),
            reason: value.reason.clone().unwrap(),
        },
        DeadlineProfileAction::Retire => DeadlineProfileChange::Retire {
            expected_revision: DeadlineProfileRevision::new(value.receipt.expected_revision)
                .unwrap(),
            reason: value.reason.clone().unwrap(),
        },
    };
    value.definition_digest =
        deadline_profile_definition_digest(inputs::hasher().as_ref(), &value.definition);
    let command = DeadlineProfileCommand {
        operation_id: value.receipt.operation_id,
        profile_id: value.id,
        change,
    };
    value.receipt.submission_digest = deadline_profile_submission_digest(
        inputs::hasher().as_ref(),
        value.recorded_by.id,
        &command,
        value.algorithm,
        value.definition_digest,
    );
    deadline_profile_receipt_matches(inputs::hasher().as_ref(), value).unwrap();
}
pub fn replace_profile(value: &mut DeadlineProfileDetail, retired: bool) {
    value.revision = DeadlineProfileRevision::new(2).unwrap();
    value.receipt.expected_revision = 1;
    value.receipt.operation_id = DeadlineProfileOperationId::from_uuid(Uuid::from_u128(505));
    value.receipt.action = if retired {
        DeadlineProfileAction::Retire
    } else {
        DeadlineProfileAction::Replace
    };
    value.status = if retired {
        DeadlineProfileStatus::Retired
    } else {
        DeadlineProfileStatus::Published
    };
    value.reason = Some(FactText::new("Observed replacement").unwrap());
    resign_profile(value);
}
pub fn calendar_pair(material: &mut DeadlineInputMaterial) {
    material.calendar = Some(inputs::calendar(
        1,
        false,
        JudicialCalendarClassification::Countable,
    ));
    material.calendar_head = Some(inputs::calendar(
        2,
        false,
        JudicialCalendarClassification::Excluded,
    ));
}
pub fn notification_material() -> (DeadlineInputMaterial, FactDetail) {
    let selected =
        DeadlineSourceDetail::Fact(Box::new(inputs::notification(1, false, 1, "2026-01-06")));
    let mut material = inputs::material(selected);
    material.source_head = Some(DeadlineSourceDetail::Fact(Box::new(inputs::notification(
        2,
        false,
        2,
        "2026-01-07",
    ))));
    (material, inputs::resolution(3, false, "2026-01-01"))
}
pub fn fact_mut(source: &mut Option<DeadlineSourceDetail>) -> &mut FactDetail {
    let Some(DeadlineSourceDetail::Fact(detail)) = source else {
        panic!("fact expected")
    };
    detail
}
pub fn fact(source: &Option<DeadlineSourceDetail>) -> &FactDetail {
    let Some(DeadlineSourceDetail::Fact(detail)) = source else {
        panic!("fact expected")
    };
    detail
}
pub fn build(
    profile: &DeadlineProfileDetail,
    material: &DeadlineInputMaterial,
    parent: Option<&FactDetail>,
) -> Result<Observations, ApplicationError> {
    build_deadline_observations(
        inputs::hasher().as_ref(),
        inputs::case_id(),
        profile,
        material,
        parent,
    )
}
pub fn entry(observations: &Observations, role: ObservationRole) -> &ObservationEntry {
    observations
        .entries
        .iter()
        .find(|entry| entry.role == role)
        .unwrap()
}
pub fn digest(
    profile: &DeadlineProfileDetail,
    material: &DeadlineInputMaterial,
    role: ObservationRole,
) -> Sha256Digest {
    entry(&build(profile, material, None).unwrap(), role).evidence_digest
}
pub fn administration(case_id: CaseId, closed: bool) -> CurrentCaseAdministration {
    let values =
        CaseAdministrationValues::basic(CaseMetadata::new("Observed case", "REF-OBS").unwrap())
            .with_status(if closed {
                CaseAdministrativeStatus::Closed
            } else {
                CaseAdministrativeStatus::Active
            });
    CurrentCaseAdministration::Recorded(Box::new(CaseAdministrationSnapshot {
        case_id,
        revision: CaseRevision::FIRST,
        values_digest: case_administration_digest(inputs::hasher().as_ref(), &values),
        values,
        changed_at: crate::case_support::instant(),
        changed_by: CaseActorSnapshot {
            id: inputs::actor(),
            email: "owner@example.test".into(),
        },
    }))
}
