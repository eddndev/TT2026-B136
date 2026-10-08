use super::{workflow_evidence as evidence, *};
use crate::{
    documents::{DocumentFormatBatchValidator, DocumentProcessor, StageSupportReadLimits},
    identity::Principal,
    precautionary_measures::{resolve_measure_closure, MeasureHistoryEvidence},
    ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};
use std::sync::Arc;

/// Constructed only after current authentication and exact encrypted support admission.
pub struct PreparedPrecautionaryHearing {
    pub(super) checked: CheckedPrecautionaryHearingReview,
    pub(super) material: PrecautionaryHearingReady,
    pub(super) merged: MeasureHistoryEvidence,
    actor: Principal,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}

impl PreparedPrecautionaryHearing {
    pub fn review(&self) -> &PrecautionaryHearingReview {
        self.checked.review()
    }
    pub fn material(&self) -> &PrecautionaryHearingReady {
        &self.material
    }
    pub fn actor(&self) -> &Principal {
        &self.actor
    }

    pub fn into_operation(
        self,
        at: OffsetDateTime,
    ) -> Result<PrecautionaryHearingStoredOperation, ApplicationError> {
        let capture = self.checked.into_capture(self.hasher.as_ref(), at)?;
        let (origin, mut captures) = match self.material.history {
            Some(history) => (history.origin, history.captures),
            None => (super::history::origin_metadata(&capture)?, Vec::new()),
        };
        captures.push(capture.clone());
        let result = PrecautionaryHearingStoredOperation {
            capture,
            history: PrecautionaryHearingHistoryEvidence {
                origin,
                captures,
                measure_history: self.merged,
            },
        };
        evidence::operation(self.hasher.as_ref(), &result)?;
        Ok(result)
    }
}

pub(super) struct PreparationServices<'a> {
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
    pub processor: &'a DocumentProcessor,
    pub validator: &'a dyn DocumentFormatBatchValidator,
    pub limits: &'a StageSupportReadLimits,
}

pub(super) fn prepare(
    services: PreparationServices<'_>,
    actor: &Principal,
    case_id: CaseId,
    command: PrecautionaryHearingCommand,
    material: PrecautionaryHearingReady,
) -> Result<PreparedPrecautionaryHearing, ApplicationError> {
    let is_schedule = command.action() == PrecautionaryHearingAction::Schedule;
    let is_cancel = command.action() == PrecautionaryHearingAction::Cancel;
    if is_schedule != material.history.is_none()
        || is_cancel == material.selected_sources.is_some()
        || material.observed_context.material().case_id != case_id
    {
        return Err(evidence::invalid(
            "preparation scope or source presence differs from command",
        ));
    }
    let hasher = services.hasher.as_ref();
    if let Some(history) = &material.history {
        if history.captures.len() >= 256 {
            return Err(PrecautionaryHearingError::IncompleteHistory.into());
        }
        evidence::history(hasher, history)?;
        let previous = history
            .captures
            .last()
            .ok_or_else(|| evidence::invalid("missing predecessor"))?;
        if previous.review.case_id != case_id
            || previous.review.command.hearing_id != command.hearing_id
            || command.expected_revision() != previous.review.result_revision.get()
            || command.predecessor() != Some(previous.capture_digest)
            || history
                .captures
                .iter()
                .any(|c| c.review.command.operation_id == command.operation_id)
        {
            return Err(evidence::invalid(
                "command differs from the exact historical predecessor",
            ));
        }
    }
    let predecessor = material.history.as_ref().and_then(|h| h.captures.last());
    let sources = match &material.selected_sources {
        Some(selected) => {
            let values = match &command.change {
                PrecautionaryHearingChange::Schedule { values, .. }
                | PrecautionaryHearingChange::Replace { values, .. } => values,
                _ => return Err(evidence::invalid("cancellation cannot admit new sources")),
            };
            if selected.participants.len() > 32 {
                return Err(evidence::invalid("too many selected participants"));
            }
            resolve_precautionary_participants(hasher, case_id, values, &selected.participants)?;
            let support = admit_precautionary_support(
                values,
                std::slice::from_ref(&selected.support_record),
                services.processor,
                services.limits,
                services.validator,
            )?;
            PrecautionaryHearingSources {
                participants: selected.participants.clone(),
                support,
            }
        }
        None => predecessor
            .ok_or_else(|| evidence::invalid("missing cancellation predecessor"))?
            .review
            .sources
            .clone(),
    };
    let checked = prepare_precautionary_hearing_with_history(
        hasher,
        actor,
        case_id,
        command,
        PrecautionaryHearingPreparationMaterial {
            observed_context: material.observed_context.clone(),
            sources,
            predecessor,
            measure_history: &material.measure_history,
        },
    )?;
    let empty = MeasureHistoryEvidence { groups: vec![] };
    let prior_proof = material
        .history
        .as_ref()
        .map(|h| &h.measure_history)
        .unwrap_or(&empty);
    let merged = evidence::merge(prior_proof, &material.measure_history)?;
    let mut reviews: Vec<_> = material
        .history
        .iter()
        .flat_map(|h| h.captures.iter().map(|c| &c.review))
        .collect();
    reviews.push(checked.review());
    let selections = measure_evidence::target_union(reviews.iter().copied())?;
    let proof = resolve_measure_closure(hasher, case_id, &selections, &merged)?;
    let mut inventory = super::source_inventory::SourceInventory::default();
    for group in proof.groups() {
        crate::precautionary_measures::add_measure_group_sources(&mut inventory, &group.review)?;
    }
    if let Some(history) = &material.history {
        for capture in &history.captures {
            inventory.capture(capture)?;
        }
    }
    inventory.add(checked.review())?;
    Ok(PreparedPrecautionaryHearing {
        checked,
        material,
        merged,
        actor: actor.clone(),
        hasher: services.hasher,
    })
}
