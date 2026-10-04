use super::{wire::invalid, *};
use crate::{
    identity::Principal,
    precautionary_hearings::{
        capture_validation::context_advances, source_inventory::SourceInventory,
        PrecautionaryContext,
    },
    precautionary_measures::{
        add_measure_group_sources, resolve_measure_sources, resolve_measure_targets,
        MeasureHistoryEvidence,
    },
    ApplicationError,
};
use domain::{
    case_administration::CaseAdministrativeStatus,
    cases::CaseId,
    clock::OffsetDateTime,
    crypto::{DocumentHasher, Sha256Digest},
    identity::Role,
};

#[derive(Debug)]
pub struct CheckedMeasureAdministrativeReview {
    pub(super) review: MeasureAdministrativeReview,
    pub(super) earliest_capture: OffsetDateTime,
}
impl CheckedMeasureAdministrativeReview {
    pub fn review(&self) -> &MeasureAdministrativeReview {
        &self.review
    }
}

/// Complete supplied ancestry is not proof of current head or absence of dependants.
pub fn prepare_measure_record_correction(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureAdministrativeCommand,
    context: PrecautionaryContext,
    history: &MeasureHistoryEvidence,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    bounds(history)?;
    if !matches!(actor.role, Role::Owner | Role::Litigator) {
        return Err(invalid("captured actor role cannot correct records"));
    }
    if history
        .groups
        .iter()
        .any(|g| g.origin.operation_id.as_uuid() == command.operation_id.as_uuid())
    {
        return Err(invalid(
            "administrative operation reuses judicial ownership",
        ));
    }
    let targets = resolve_measure_targets(hasher, case_id, &[command.target], history)?;
    let previous = targets
        .targets()
        .first()
        .ok_or_else(|| invalid("missing exact predecessor"))?;
    let group = history
        .groups
        .iter()
        .find(|entry| entry.origin.operation_id == previous.owner.operation_id)
        .ok_or_else(|| invalid("missing last judicial owner"))?;
    PrecautionaryContext::new(hasher, context.material().clone())?;
    let observed = context.material();
    if observed.case_id != case_id
        || observed.administration.values.status() != CaseAdministrativeStatus::Active
        || command.context.administration_revision != observed.administration.revision
        || command.context.stage_revision != observed.stage.stage_revision()
        || command.context.context_digest != context.digest(hasher)
    {
        return Err(invalid("active exact context differs"));
    }
    context_advances(&group.capture.review.material.context, &context)?;
    let prior = &previous.capture;
    let revision = prior
        .result
        .revision
        .next()
        .ok_or_else(|| invalid("measure revision overflow"))?;
    if history.groups.iter().any(|g| {
        g.capture
            .measures
            .iter()
            .any(|m| m.result.id == prior.result.id && m.result.revision == revision)
    }) {
        return Err(invalid("administrative result already has an owner"));
    }
    let values = match &command.action {
        MeasureAdministrativeAction::Correct(values) => {
            prior.result.values.correct_record(values)?
        }
    };
    let projection = resolve_measure_sources(hasher, case_id, &values, &prior.result.sources)?;
    let mut inventory = SourceInventory::default();
    for group in &history.groups {
        add_measure_group_sources(&mut inventory, &group.capture.review)?;
    }
    inventory.context(&context)?;
    let earliest_capture = prior
        .recorded_at
        .max(observed.administration.changed_at)
        .max(observed.stage.recorded_at())
        .max(observed.stage_administration.changed_at);
    let result = MeasureAdministrativeResult {
        id: prior.result.id,
        revision,
        previous: command.target,
        record_root: MeasureRecordRoot::Judicial(prior.result.origin),
        judicial_origin: prior.result.origin,
        last_judicial: MeasureJudicialRef {
            owner: previous.owner.clone(),
            reference: command.target,
        },
        last_action: prior.result.action,
        validity: MeasureCaptureValidity::Valid,
        values,
        sources: prior.result.sources.clone(),
        projection,
    };
    let submission_digest = hasher.hash_bytes(&measure_administrative_submission_bytes(
        actor, case_id, &command,
    )?);
    let mut review = MeasureAdministrativeReview {
        case_id,
        actor: actor.clone(),
        command,
        context,
        support: group.capture.decision.support.clone(),
        result,
        submission_digest,
        review_digest: Sha256Digest::from_array([0; 32]),
    };
    review.review_digest = hasher.hash_bytes(&measure_administrative_review_bytes(&review)?);
    Ok(CheckedMeasureAdministrativeReview {
        review,
        earliest_capture,
    })
}

fn bounds(history: &MeasureHistoryEvidence) -> Result<(), ApplicationError> {
    if history.groups.len() >= 256 {
        return Err(invalid("combined owner budget exceeded"));
    }
    let mut rows = 1usize;
    for group in &history.groups {
        rows = rows
            .checked_add(group.capture.measures.len())
            .ok_or_else(|| invalid("row count overflow"))?;
        if rows > 8192 {
            return Err(invalid("combined measure row budget exceeded"));
        }
    }
    Ok(())
}
