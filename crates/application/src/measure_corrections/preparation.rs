use super::{wire::invalid, *};
use crate::{
    identity::Principal,
    precautionary_hearings::{
        capture_validation::context_advances, source_inventory::SourceInventory,
        PrecautionaryContext,
    },
    precautionary_measures::{resolve_measure_sources, MeasureHistoryEvidence},
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
    require_correction(&command)?;
    prepare_with_view(
        hasher,
        actor,
        case_id,
        command,
        context,
        super::record_index::HistoryView {
            judicial: history,
            administrative: &[],
        },
    )
}

pub fn prepare_measure_record_correction_with_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureAdministrativeCommand,
    context: PrecautionaryContext,
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    require_correction(&command)?;
    prepare_measure_administrative_record_with_history(
        hasher, actor, case_id, command, context, evidence,
    )
}

fn require_correction(command: &MeasureAdministrativeCommand) -> Result<(), ApplicationError> {
    if !matches!(command.action, MeasureAdministrativeAction::Correct(_)) {
        return Err(invalid("correction entry point requires correction values"));
    }
    Ok(())
}

/// Captures declared recording validity without asserting a judicial effect or live eligibility.
pub fn prepare_measure_administrative_record_with_history(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureAdministrativeCommand,
    context: PrecautionaryContext,
    evidence: &MeasureRecordHistoryEvidence,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    prepare_with_view(hasher, actor, case_id, command, context, evidence.into())
}

pub(super) fn prepare_with_view(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureAdministrativeCommand,
    context: PrecautionaryContext,
    evidence: super::record_index::HistoryView<'_>,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    let mut inventory = SourceInventory::default();
    let index = super::record_history::validate(
        hasher,
        case_id,
        &[command.target],
        evidence,
        1,
        &mut inventory,
    )?;
    index.candidate(&command)?;
    let previous = index.view(index.selected(command.target)?)?;
    let checked = prepare_from_record(hasher, actor, case_id, command, context, previous)?;
    super::record_history::add_sources(&mut inventory, &checked.review)?;
    Ok(checked)
}

pub(super) fn prepare_from_record(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case_id: CaseId,
    command: MeasureAdministrativeCommand,
    context: PrecautionaryContext,
    previous: super::record_view::RecordView<'_>,
) -> Result<CheckedMeasureAdministrativeReview, ApplicationError> {
    if !matches!(actor.role, Role::Owner | Role::Litigator) {
        return Err(invalid("captured actor role cannot correct records"));
    }
    if previous.validity() != MeasureCaptureValidity::Valid {
        return Err(invalid("entered-in-error record cannot be corrected"));
    }
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
    context_advances(previous.context(), &context)?;
    let revision = previous
        .reference()
        .revision()
        .next()
        .ok_or_else(|| invalid("measure revision overflow"))?;
    let (values, validity) = match &command.action {
        MeasureAdministrativeAction::Correct(values) => (
            previous.values().correct_record(values)?,
            MeasureCaptureValidity::Valid,
        ),
        MeasureAdministrativeAction::MarkEnteredInError => (
            previous.values().clone(),
            MeasureCaptureValidity::EnteredInError,
        ),
    };
    let projection = resolve_measure_sources(hasher, case_id, &values, previous.sources())?;
    let earliest_capture = previous
        .recorded_at()
        .max(observed.administration.changed_at)
        .max(observed.stage.recorded_at())
        .max(observed.stage_administration.changed_at);
    let result = MeasureAdministrativeResult {
        id: previous.reference().id(),
        revision,
        previous: command.target,
        record_root: previous.record_root(),
        judicial_origin: previous.judicial().result.origin,
        last_judicial: previous.judicial_reference(),
        last_action: previous.judicial().result.action,
        validity,
        values,
        sources: previous.sources().clone(),
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
        support: previous.support().clone(),
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
