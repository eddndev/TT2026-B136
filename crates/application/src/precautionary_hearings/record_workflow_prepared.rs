use super::{record_workflow_evidence as evidence, workflow_evidence::invalid, *};
use crate::{
    identity::Principal, measure_corrections::checked_record_closure,
    precautionary_measures::MeasureDecisionRecordHistoryEvidence, ApplicationError,
};
use domain::{cases::CaseId, clock::OffsetDateTime, crypto::DocumentHasher};
use std::sync::Arc;

/// Created only after authentication, exact historical proof and document admission.
pub struct PreparedPrecautionaryHearingRecord {
    pub(super) checked: CheckedPrecautionaryHearingReview,
    material: PrecautionaryHearingRecordReady,
    pub(super) merged: MeasureDecisionRecordHistoryEvidence,
    actor: Principal,
    hasher: Arc<dyn DocumentHasher + Send + Sync>,
}

impl PreparedPrecautionaryHearingRecord {
    pub fn actor(&self) -> &Principal {
        &self.actor
    }
    pub fn review(&self) -> &PrecautionaryHearingReview {
        self.checked.review()
    }
    pub fn material(&self) -> &PrecautionaryHearingRecordReady {
        &self.material
    }

    pub fn into_operation(
        self,
        at: OffsetDateTime,
    ) -> Result<PrecautionaryHearingRecordStoredOperation, ApplicationError> {
        let capture = self.checked.into_capture(self.hasher.as_ref(), at)?;
        let (origin, mut captures) = match self.material.history {
            Some(history) => (history.origin, history.captures),
            None => (super::history::origin_metadata(&capture)?, vec![]),
        };
        captures.push(capture.clone());
        let result = PrecautionaryHearingRecordStoredOperation {
            capture,
            history: PrecautionaryHearingRecordHistoryEvidence {
                origin,
                captures,
                record_history: self.merged,
            },
        };
        evidence::operation(self.hasher.as_ref(), &result)?;
        Ok(result)
    }
}

pub(super) fn prepare(
    services: super::workflow_prepared::PreparationServices<'_>,
    actor: &Principal,
    case_id: CaseId,
    command: PrecautionaryHearingCommand,
    material: PrecautionaryHearingRecordReady,
) -> Result<PreparedPrecautionaryHearingRecord, ApplicationError> {
    evidence::ready_bounds(&material)?;
    let schedule = command.action() == PrecautionaryHearingAction::Schedule;
    let cancel = command.action() == PrecautionaryHearingAction::Cancel;
    if schedule != material.history.is_none()
        || cancel == material.selected_sources.is_some()
        || material.observed_context.material().case_id != case_id
    {
        return Err(invalid(
            "preparation scope or source presence differs from command",
        ));
    }
    let hasher = services.hasher.as_ref();
    if let Some(history) = &material.history {
        evidence::history(hasher, history)?;
        let previous = history
            .captures
            .last()
            .ok_or_else(|| invalid("missing predecessor"))?;
        if previous.review.case_id != case_id
            || previous.review.command.hearing_id != command.hearing_id
            || command.expected_revision() != previous.review.result_revision.get()
            || command.predecessor() != Some(previous.capture_digest)
            || history
                .captures
                .iter()
                .any(|c| c.review.command.operation_id == command.operation_id)
        {
            return Err(invalid(
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
                _ => return Err(invalid("cancellation cannot admit new sources")),
            };
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
            .ok_or_else(|| invalid("missing cancellation predecessor"))?
            .review
            .sources
            .clone(),
    };
    let checked = prepare_precautionary_hearing_with_decision_history(
        hasher,
        actor,
        case_id,
        command,
        PrecautionaryHearingDecisionPreparationMaterial {
            observed_context: material.observed_context.clone(),
            sources,
            predecessor,
            decision_history: &material.record_history,
        },
    )?;
    let empty = evidence::empty();
    let prior_proof = material
        .history
        .as_ref()
        .map(|h| &h.record_history)
        .unwrap_or(&empty);
    let merged = evidence::merge(prior_proof, &material.record_history)?;
    check_shared(hasher, case_id, &material, checked.review(), &merged)?;
    Ok(PreparedPrecautionaryHearingRecord {
        checked,
        material,
        merged,
        actor: actor.clone(),
        hasher: services.hasher,
    })
}

fn check_shared(
    hasher: &dyn DocumentHasher,
    case_id: CaseId,
    material: &PrecautionaryHearingRecordReady,
    review: &PrecautionaryHearingReview,
    merged: &MeasureDecisionRecordHistoryEvidence,
) -> Result<(), ApplicationError> {
    let captures = material.history.iter().flat_map(|h| h.captures.iter());
    let selections = measure_evidence::target_union(
        captures
            .clone()
            .map(|c| &c.review)
            .chain(std::iter::once(review)),
    )?;
    let mut inventory = source_inventory::SourceInventory::default();
    checked_record_closure(hasher, case_id, &selections, merged.into(), &mut inventory)?;
    for capture in captures {
        inventory.capture(capture)?;
    }
    inventory.add(review)
}
