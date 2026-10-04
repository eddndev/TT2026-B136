use super::*;
use crate::{
    deadline_inputs::{checked_calendar, DeadlineSourceDetail},
    deadline_observations::verify_captured_deadline_observations,
    deadline_profiles::deadline_profile_receipt_matches,
    deadlines::{evidence, validate_profile_selection, *},
};
use domain::{crypto::DocumentHasher, identity::Permission};

pub(super) fn validate(
    hasher: &dyn DocumentHasher,
    value: &HearingDerivedDeadlineEvidence,
) -> Result<(), ApplicationError> {
    let actor = &value.actor;
    if !actor.role.allows(Permission::ManageHearingResult)
        || !actor.role.allows(Permission::ManageDeadline)
    {
        return Err(invalid("derived.history.actor"));
    }
    recorded_source::validate_parts(
        hasher,
        actor,
        &value.command.result,
        &value.material.result,
        &value.result,
        value.source_event,
    )?;
    let deadline = &value.deadline;
    deadline_receipt_matches(hasher, deadline)?;
    let source = &value.result.snapshot;
    let tracking = deadline
        .tracking
        .as_ref()
        .ok_or_else(|| invalid("derived.history.tracking"))?;
    if deadline.case_id != source.case_id
        || deadline.revision != DeadlineRevision::initial()
        || deadline.receipt.action != DeadlineAction::Register
        || deadline.recorded_by
            != (DeadlineActorSnapshot::User {
                id: actor.id,
                email: actor.email.clone(),
            })
        || deadline.recorded_at != source.recorded_at
        || deadline.recorded_at.offset() != source.recorded_at.offset()
    {
        return Err(invalid("derived.history.pair"));
    }
    let material = &value.material;
    deadline_profile_receipt_matches(hasher, &material.profile)?;
    deadline_profile_receipt_matches(hasher, &material.profile_head)?;
    validate_profile_selection(
        &material.profile,
        &material.profile_head,
        Some(tracking.policies.profile),
    )?;
    checked_calendar(
        hasher,
        deadline.definition.input.calendar,
        material.calendar.as_ref(),
        material.calendar_head.as_ref(),
    )?;
    verify_captured_deadline_observations(
        hasher,
        &tracking.observations,
        &material.profile_head,
        &deadline.calculation.material,
        None,
    )?;
    let actual_source = DeadlineSourceDetail::HearingResult(Box::new(value.result.clone()));
    let encode_source = |source| {
        let mut bytes = Vec::new();
        evidence::source(&mut bytes, hasher, source);
        bytes
    };
    for stored in [
        &deadline.calculation.material.source,
        &deadline.calculation.material.source_head,
    ] {
        if encode_source(stored.as_ref()) != encode_source(Some(&actual_source)) {
            return Err(invalid("derived.history.source"));
        }
    }
    let encode_admin = |admin| {
        let mut bytes = Vec::new();
        evidence::administration(&mut bytes, hasher, admin);
        bytes
    };
    if encode_admin(&tracking.administration)
        != encode_admin(&deadline.calculation.material.administration)
    {
        return Err(invalid("derived.history.administration"));
    }
    let mut captured = material.clone();
    captured.result.observed_administration = deadline.calculation.material.administration.clone();
    captured.profile = deadline.calculation.profile.clone();
    captured.calendar = deadline.calculation.material.calendar.clone();
    captured.calendar_head = deadline.calculation.material.calendar_head.clone();
    captured.responsible = deadline.responsible.clone();
    let captured_command = HearingDerivedDeadlineCommand {
        result: value.command.result.clone(),
        deadline: DeadlineHumanCommand::new(
            DeadlineCommand {
                operation_id: deadline.receipt.operation_id,
                deadline_id: deadline.id,
                change: DeadlineChange::Register {
                    definition: deadline.definition.clone(),
                },
            },
            Some(tracking.policies),
        )?,
    };
    let reviewed = canonical::review_bytes(
        hasher,
        actor,
        &value.command,
        material,
        &deadline.calculation.result,
    )?;
    let actual = canonical::review_bytes(
        hasher,
        actor,
        &captured_command,
        &captured,
        &deadline.calculation.result,
    )?;
    if reviewed != actual {
        return Err(DeadlineError::SubmissionMismatch.into());
    }
    Ok(())
}
