use super::{
    decision_wire::{bounded, invalid},
    record_decision_wire::reference_of_record,
    *,
};
use crate::{
    identity::Principal,
    measure_corrections::{MeasureRecordRoot, RecordView},
    precautionary_hearings::{
        capture_validation::context_advances, source_inventory::SourceInventory,
        PrecautionaryContext,
    },
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

/// Checked supplied records, not proof of current authority, admission or durable heads.
#[derive(Debug)]
pub struct CheckedMeasureDecisionReviewV2 {
    pub(super) review: MeasureDecisionReviewV2,
    pub(super) earliest_capture: OffsetDateTime,
}

impl CheckedMeasureDecisionReviewV2 {
    pub fn review(&self) -> &MeasureDecisionReviewV2 {
        &self.review
    }
}

pub(crate) fn prepare_record_flat(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureDecisionCommand,
    mut material: MeasureDecisionMaterialV2,
    predecessors: &[RecordView<'_>],
    anchor_targets: &[RecordView<'_>],
) -> Result<CheckedMeasureDecisionReviewV2, ApplicationError> {
    bounded(material.result_sources.len())?;
    bounded(material.predecessors.len())?;
    bounded(predecessors.len())?;
    bounded(anchor_targets.len())?;
    super::anchor_validation::shape(&material.anchor)?;
    let mut predecessors = predecessors.to_vec();
    predecessors.sort_by_key(|prior| prior.reference().id().as_uuid());
    material
        .predecessors
        .sort_by_key(|prior| reference_of_record(prior).id().as_uuid());
    if predecessors.len() != material.predecessors.len() {
        return Err(invalid("supplied predecessor count differs from proof"));
    }
    for (prior, supplied) in predecessors.iter().zip(&material.predecessors) {
        if RecordView::to_owned(*prior).record() != supplied {
            return Err(invalid("supplied predecessor differs from owning capture"));
        }
        context_advances(prior.context(), &material.context)?;
    }
    let anchor_time = super::record_decision_anchor::validate(
        hasher,
        case_id,
        &command,
        &material,
        anchor_targets,
    )?;
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
    let support = command.values.support();
    if support.reference() != material.support.reference
        || support.digest() != material.support.digest
    {
        return Err(invalid(
            "decision support differs from exact selected version",
        ));
    }
    ArchiveEntry::new(material.support.name.clone(), Vec::new())?;
    material
        .result_sources
        .sort_by_key(|item| item.id.as_uuid());
    let effects = super::record_decision_effects::effects(&command, &predecessors)?;
    if effects.len() != material.result_sources.len() {
        return Err(invalid(
            "result source inventory differs from declared effects",
        ));
    }
    let mut earliest_capture = context
        .administration
        .changed_at
        .max(context.stage.recorded_at())
        .max(context.stage_administration.changed_at);
    if let Some(at) = anchor_time {
        earliest_capture = earliest_capture.max(at);
    }
    let initial_origin = MeasureOriginIds {
        decision_id: command.decision_id,
        operation_id: command.operation_id,
    };
    let mut results = Vec::with_capacity(effects.len());
    for (effect, sources) in effects.iter().zip(&material.result_sources) {
        if sources.id != effect.id {
            return Err(invalid("measure source identity differs"));
        }
        if let Some(prior) = effect.prior {
            earliest_capture = earliest_capture.max(prior.recorded_at());
            if effect.action != MeasureCaptureAction::Modify && &sources.sources != prior.sources()
            {
                return Err(invalid(
                    "unchanged declarations must retain complete prior sources",
                ));
            }
        }
        let projection = resolve_measure_sources(hasher, case_id, effect.values, &sources.sources)?;
        if let Some(prior) = effect.prior {
            if effect.action != MeasureCaptureAction::Modify && &projection != prior.projection() {
                return Err(invalid(
                    "unchanged declarations must retain the prior projection",
                ));
            }
        }
        earliest_capture = earliest_capture.max(sources.sources.subject.changed_at);
        if let Some(supervisor) = &sources.sources.supervisor {
            let at = match &supervisor.revision {
                ParticipantRevisionSnapshot::Manual(value) => value.changed_at,
                ParticipantRevisionSnapshot::Typed(value) => value.changed_at,
            };
            earliest_capture = earliest_capture.max(at);
            if let Some(subject) = &supervisor.bound_subject {
                earliest_capture = earliest_capture.max(subject.changed_at);
            }
        }
        results.push(ReviewedMeasureResultV2 {
            id: effect.id,
            revision: super::record_decision_effects::revision(effect.prior)?,
            record_root: effect.prior.map_or_else(
                || MeasureRecordRoot::Judicial(initial_origin),
                |prior| prior.record_root(),
            ),
            judicial_origin: effect
                .prior
                .map_or(initial_origin, |prior| prior.judicial_origin()),
            effect_key: effect.effect_key,
            action: effect.action,
            previous: effect.prior.map(|prior| prior.reference()),
            values: effect.values.clone(),
            sources: sources.sources.clone(),
            projection,
        });
    }
    let submission_digest = hasher.hash_bytes(&measure_decision_submission_bytes(
        actor, case_id, &command,
    )?);
    let mut review = MeasureDecisionReviewV2 {
        case_id,
        actor: actor.clone(),
        command,
        material,
        results,
        submission_digest,
        review_digest: Sha256Digest::from_array([0; 32]),
    };
    add_record_decision_sources(&mut SourceInventory::default(), &review)?;
    review.review_digest = hasher.hash_bytes(&measure_decision_review_v2_bytes(&review)?);
    Ok(CheckedMeasureDecisionReviewV2 {
        review,
        earliest_capture,
    })
}

pub(crate) fn add_record_decision_sources<'a>(
    inventory: &mut SourceInventory<'a>,
    review: &'a MeasureDecisionReviewV2,
) -> Result<(), ApplicationError> {
    inventory.context(&review.material.context)?;
    inventory.support(&review.material.support)?;
    if let Some(anchor) = &review.material.anchor {
        inventory.anchor(anchor)?;
    }
    for result in &review.results {
        inventory.subject(&result.sources.subject)?;
        if let Some(supervisor) = &result.sources.supervisor {
            inventory.participant(supervisor)?;
        }
        if let Some(supervisor) = &result.projection.supervisor {
            inventory.projection(&supervisor.overview, supervisor.snapshot.values_digest)?;
        }
    }
    Ok(())
}
