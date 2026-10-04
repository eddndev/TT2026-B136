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
    precautionary_hearings::MeasureRevision,
    precautionary_measures::MeasureEffect,
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
    mut material: MeasureDecisionMaterial,
) -> Result<CheckedMeasureDecisionReview, ApplicationError> {
    bounded(material.result_sources.len())?;
    if command.anchor.is_some() || material.anchor.is_some() || !material.predecessors.is_empty() {
        return Err(invalid(
            "anchor or predecessor evidence is not yet supported",
        ));
    }
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
    let effects = command.outcome.changes().unwrap_or(&[]);
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
    let mut results = Vec::with_capacity(effects.len());
    for (effect, material) in effects.iter().zip(&material.result_sources) {
        let MeasureEffect::Impose(proposal) = effect else {
            return Err(invalid(
                "existing measure effects require verified owning histories",
            ));
        };
        if material.id != proposal.id {
            return Err(invalid("measure source identity differs"));
        }
        let projection =
            resolve_measure_sources(hasher, case_id, &proposal.values, &material.sources)?;
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
            id: proposal.id,
            revision: MeasureRevision::initial(),
            origin: MeasureOriginIds {
                decision_id: command.decision_id,
                operation_id: command.operation_id,
            },
            effect_key: proposal.id,
            action: MeasureCaptureAction::Impose,
            previous: None,
            values: proposal.values.clone(),
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
