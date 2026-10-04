use super::*;
use crate::{typed_participants::ParticipantRevisionSnapshot, ApplicationError};
use domain::{
    case_administration::CaseAdministrativeStatus,
    clock::OffsetDateTime,
    crypto::{ArchiveEntry, DocumentHasher},
    hearings::HearingStatus,
    identity::Role,
    precautionary_hearings::PrecautionaryHearingPurpose,
};

pub(super) fn invalid(message: &str) -> ApplicationError {
    ApplicationError::InvalidInput(format!("inconsistent precautionary capture: {message}"))
}

pub(super) fn clock(at: OffsetDateTime) -> Result<(), ApplicationError> {
    if at.offset() != time::UtcOffset::UTC || !(1..=9999).contains(&at.year()) {
        return Err(invalid("capture provenance must use supported UTC"));
    }
    Ok(())
}

pub(crate) fn context_advances(
    previous: &PrecautionaryContext,
    next: &PrecautionaryContext,
) -> Result<(), ApplicationError> {
    let a = previous.material();
    let b = next.material();
    if a.case_id != b.case_id
        || b.administration.revision < a.administration.revision
        || b.administration.changed_at < a.administration.changed_at
        || (a.administration.revision == b.administration.revision
            && a.administration != b.administration)
        || b.stage.stage_revision() < a.stage.stage_revision()
        || b.stage.recorded_at() < a.stage.recorded_at()
        || (a.stage.stage_revision() == b.stage.stage_revision() && a.stage != b.stage)
    {
        return Err(invalid(
            "observed context regressed or changed at one revision",
        ));
    }
    Ok(())
}

pub(super) fn validate_review(
    hasher: &dyn DocumentHasher,
    review: &PrecautionaryHearingReview,
) -> Result<(), ApplicationError> {
    if !matches!(review.actor.role, Role::Owner | Role::Litigator) {
        return Err(invalid("captured actor role cannot record appointments"));
    }
    if review.resolved_values.purpose() != PrecautionaryHearingPurpose::Imposition {
        return Err(invalid(
            "review appointments require verified measure captures",
        ));
    }
    for context in [&review.scheduling_context, &review.observed_context] {
        let material = context.material();
        PrecautionaryContext::new(hasher, material.clone())?;
        if material.case_id != review.case_id
            || material.administration.values.status() != CaseAdministrativeStatus::Active
        {
            return Err(invalid("capture context is foreign or inactive"));
        }
    }
    context_advances(&review.scheduling_context, &review.observed_context)?;
    if review.result_revision != review.command.result_revision()? {
        return Err(invalid("result revision differs from instruction"));
    }
    let status = if review.command.action() == PrecautionaryHearingAction::Cancel {
        HearingStatus::Cancelled
    } else {
        let material = review.observed_context.material();
        let expected = review
            .command
            .context()
            .ok_or_else(|| invalid("missing expected context"))?;
        if expected.administration_revision != material.administration.revision
            || expected.stage_revision != material.stage.stage_revision()
            || expected.context_digest != review.observed_context.digest(hasher)
            || review.scheduling_context != review.observed_context
        {
            return Err(invalid("scheduling context differs from review"));
        }
        HearingStatus::Scheduled
    };
    if status != review.status {
        return Err(invalid("status differs from instruction"));
    }
    if review
        .sources
        .participants
        .windows(2)
        .any(|pair| pair[0].id().as_uuid() >= pair[1].id().as_uuid())
    {
        return Err(invalid("participant sources are not uniquely ordered"));
    }
    let projected = resolve_precautionary_participants(
        hasher,
        review.case_id,
        &review.resolved_values,
        &review.sources.participants,
    )?;
    if projected != review.participants {
        return Err(invalid("participant projection differs from full material"));
    }
    let support = &review.sources.support;
    let selected = review.resolved_values.scheduling_basis().support();
    if selected.reference() != support.reference || selected.digest() != support.digest {
        return Err(invalid("support differs from exact declared version"));
    }
    ArchiveEntry::new(support.name.clone(), Vec::new())?;
    source_inventory::SourceInventory::default().add(review)?;
    latest_source_time(review)?;
    let instruction = precautionary_hearing_submission_bytes(
        &review.actor,
        review.case_id,
        &review.command,
        &review.resolved_values,
    )?;
    if hasher.hash_bytes(&instruction) != review.submission_digest
        || hasher.hash_bytes(&precautionary_hearing_review_bytes(review)?) != review.review_digest
    {
        return Err(invalid("instruction or review commitment differs"));
    }
    Ok(())
}

pub(super) fn latest_source_time(
    review: &PrecautionaryHearingReview,
) -> Result<OffsetDateTime, ApplicationError> {
    let mut latest = review.observed_context.material().administration.changed_at;
    for context in [&review.scheduling_context, &review.observed_context] {
        let m = context.material();
        latest = latest
            .max(m.administration.changed_at)
            .max(m.stage.recorded_at())
            .max(m.stage_administration.changed_at);
    }
    for detail in &review.sources.participants {
        let (at, actor) = match &detail.revision {
            ParticipantRevisionSnapshot::Manual(source) => (source.changed_at, &source.changed_by),
            ParticipantRevisionSnapshot::Typed(source) => (source.changed_at, &source.changed_by),
        };
        clock(at)?;
        super::encoding::validate_actor_email(&actor.email)?;
        latest = latest.max(at);
        if let Some(subject) = &detail.bound_subject {
            clock(subject.changed_at)?;
            super::encoding::validate_actor_email(&subject.changed_by.email)?;
            latest = latest.max(subject.changed_at);
        }
    }
    Ok(latest)
}

/// Verifies one flat capture, not the existence of its predecessor or durable origin.
pub fn precautionary_hearing_receipt_matches(
    hasher: &dyn DocumentHasher,
    capture: &PrecautionaryHearingCapture,
) -> Result<(), ApplicationError> {
    validate_review(hasher, &capture.review)?;
    clock(capture.recorded_at)?;
    if capture.recorded_at < latest_source_time(&capture.review)?
        || hasher.hash_bytes(&precautionary_hearing_capture_bytes(capture)?)
            != capture.capture_digest
    {
        return Err(invalid("capture clock or commitment differs"));
    }
    Ok(())
}

/// Verifies a flat adjacent pair; complete ancestry requires a separate history check.
pub fn precautionary_hearing_transition_matches(
    hasher: &dyn DocumentHasher,
    previous: &PrecautionaryHearingCapture,
    next: &PrecautionaryHearingCapture,
) -> Result<(), ApplicationError> {
    precautionary_hearing_receipt_matches(hasher, previous)?;
    precautionary_hearing_receipt_matches(hasher, next)?;
    transition(previous, &next.review)?;
    if next.recorded_at < previous.recorded_at {
        return Err(invalid("capture predates its predecessor"));
    }
    Ok(())
}

pub(super) fn transition(
    previous: &PrecautionaryHearingCapture,
    next: &PrecautionaryHearingReview,
) -> Result<(), ApplicationError> {
    let prior = &previous.review;
    if prior.case_id != next.case_id
        || prior.command.hearing_id != next.command.hearing_id
        || prior.status != HearingStatus::Scheduled
        || prior.command.operation_id == next.command.operation_id
        || next.command.expected_revision() != prior.result_revision.get()
        || next.command.predecessor() != Some(previous.capture_digest)
    {
        return Err(invalid("exact scheduled predecessor differs"));
    }
    context_advances(&prior.observed_context, &next.observed_context)?;
    let mut inventory = source_inventory::SourceInventory::default();
    inventory.add(prior)?;
    inventory.add(next)?;
    if next.command.action() == PrecautionaryHearingAction::Cancel
        && (next.resolved_values != prior.resolved_values
            || next.scheduling_context != prior.scheduling_context
            || next.sources != prior.sources
            || next.participants != prior.participants)
    {
        return Err(invalid("cancellation changed retained scheduling evidence"));
    }
    Ok(())
}
