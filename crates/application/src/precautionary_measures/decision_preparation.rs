use super::{
    decision_wire::{bounded, invalid},
    *,
};
use crate::{
    identity::Principal,
    precautionary_hearings::{source_inventory::SourceInventory, PrecautionaryContext},
    typed_participants::ParticipantRevisionSnapshot,
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{ArchiveEntry, DocumentHasher, Sha256Digest},
    identity::Role,
};

/// Checked supplied declaration, not proof of authorization, admission or persistence.
#[derive(Debug)]
pub struct CheckedMeasureDecisionReview {
    pub(super) review: MeasureDecisionReview,
    pub(super) earliest_capture: OffsetDateTime,
}

impl CheckedMeasureDecisionReview {
    pub fn review(&self) -> &MeasureDecisionReview {
        &self.review
    }
}

/// Prepares standalone initial impositions or an explicit decision without measure changes.
/// Linked decisions and predecessor effects require their separate complete evidence.
pub fn prepare_measure_decision_capture(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    material: MeasureDecisionMaterial,
) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
    prepare_measure_decision_with_history(
        hasher,
        actor,
        case_id,
        command,
        material,
        &MeasureHistoryEvidence { groups: vec![] },
    )
}

pub(super) fn prepare_flat(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    mut material: MeasureDecisionMaterial,
    proof: &CheckedMeasureTargets<'_>,
) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
    bounded(material.result_sources.len())?;
    bounded(material.predecessors.len())?;
    let anchor_time =
        super::anchor_validation::validate(hasher, case_id, &command, &material, proof)?;
    if !matches!(actor.role, Role::Owner | Role::Litigator) {
        return Err(invalid("captured actor role cannot record decisions"));
    }
    let context = material.context.material();
    PrecautionaryContext::new(hasher, context.clone())?;
    if context.case_id != case_id
        || context.administration.values.status() != CaseAdministrativeStatus::Active
        || command.context.administration_revision != context.administration.revision
        || command.context.stage_revision != context.stage.stage_revision()
        || command.context.context_digest != material.context.digest(hasher)
    {
        return Err(invalid(
            "decision context differs from exact active selection",
        ));
    }
    let selected = command.values.support();
    if selected.reference() != material.support.reference
        || selected.digest() != material.support.digest
    {
        return Err(invalid(
            "decision support differs from exact selected version",
        ));
    }
    ArchiveEntry::new(material.support.name.clone(), Vec::new())?;
    material
        .result_sources
        .sort_by_key(|item| item.id.as_uuid());
    material
        .predecessors
        .sort_by_key(|item| item.capture.result.id.as_uuid());
    let effects = super::effect_resolution::effects(&command, &material)?;
    if effects.len() != material.result_sources.len() {
        return Err(invalid(
            "result source inventory differs from declared effects",
        ));
    }
    let mut inventory = SourceInventory::default();
    inventory.context(&material.context)?;
    inventory.support(&material.support)?;
    let mut earliest_capture = context
        .administration
        .changed_at
        .max(context.stage.recorded_at())
        .max(context.stage_administration.changed_at);
    if let Some(at) = anchor_time {
        earliest_capture = earliest_capture.max(at);
    }
    let mut results = Vec::with_capacity(effects.len());
    for (effect, material) in effects.iter().zip(&material.result_sources) {
        if material.id != effect.id {
            return Err(invalid("measure source identity differs"));
        }
        if let Some(prior) = effect.prior {
            earliest_capture = earliest_capture.max(prior.recorded_at);
            if effect.action != MeasureCaptureAction::Modify
                && material.sources != prior.result.sources
            {
                return Err(invalid(
                    "unchanged declarations must retain complete prior sources",
                ));
            }
        }
        let projection =
            resolve_measure_sources(hasher, case_id, effect.values, &material.sources)?;
        inventory.subject(&material.sources.subject)?;
        earliest_capture = earliest_capture.max(material.sources.subject.changed_at);
        if let Some(supervisor) = &material.sources.supervisor {
            inventory.participant(supervisor)?;
            let at = match &supervisor.revision {
                ParticipantRevisionSnapshot::Manual(value) => value.changed_at,
                ParticipantRevisionSnapshot::Typed(value) => value.changed_at,
            };
            earliest_capture = earliest_capture.max(at);
            if let Some(subject) = &supervisor.bound_subject {
                earliest_capture = earliest_capture.max(subject.changed_at);
            }
        }
        results.push(ReviewedMeasureResult {
            id: effect.id,
            revision: super::effect_resolution::revision(effect.prior)?,
            origin: effect
                .prior
                .map(|prior| prior.result.origin)
                .unwrap_or(MeasureOriginIds {
                    decision_id: command.decision_id,
                    operation_id: command.operation_id,
                }),
            effect_key: effect.effect_key,
            action: effect.action,
            previous: effect
                .prior
                .map(super::effect_resolution::capture_reference),
            values: effect.values.clone(),
            sources: material.sources.clone(),
            projection,
        });
    }
    let submission_digest = hasher.hash_bytes(&measure_decision_submission_bytes(
        actor, case_id, &command,
    )?);
    let mut review = MeasureDecisionReview {
        case_id,
        actor: actor.clone(),
        command,
        material,
        results,
        submission_digest,
        review_digest: Sha256Digest::from_array([0; 32]),
    };
    review.review_digest = hasher.hash_bytes(&measure_decision_review_bytes(&review)?);
    Ok(CheckedMeasureDecisionReview {
        review,
        earliest_capture,
    })
}
