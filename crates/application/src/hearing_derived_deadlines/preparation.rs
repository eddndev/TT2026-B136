use super::*;
use crate::{
    deadline_evaluations::{evaluate_extracted_deadline, DeadlineEvaluationError},
    deadline_inputs::checked_calendar,
    deadline_profiles::deadline_profile_receipt_matches,
    deadline_tracking::TrackingPolicy,
    deadlines::{evidence, validate_profile_selection, DeadlineChange},
    hearing_results::validate_record_draft,
    judicial_calendars::JudicialCalendarStatus,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    deadline_triggers::{
        extract_trigger_time, HearingTriggerDigests, TriggerFamily, TriggerMaterial,
        TriggerSourceRef,
    },
    identity::Permission,
    procedural_facts::FactDeclaration,
};
use time::OffsetDateTime;

/// Review a consequence before its source exists. Actual storage must reauthenticate,
/// resolve current access and inputs, and capture both records atomically.
pub fn prepare_hearing_derived_deadline(
    hasher: &dyn DocumentHasher,
    actor: &Principal,
    case: CaseId,
    command: HearingDerivedDeadlineCommand,
    material: HearingDerivedDeadlineMaterial,
    observed_at: OffsetDateTime,
) -> Result<HearingDerivedDeadlineDraft, ApplicationError> {
    if !actor.role.allows(Permission::ManageHearingResult)
        || !actor.role.allows(Permission::ManageDeadline)
    {
        return Err(ApplicationError::PermissionDenied);
    }
    if actor.email.trim().is_empty() {
        return Err(invalid("derived.actor"));
    }
    validate_record_draft(
        hasher,
        actor.id,
        case,
        &command.result,
        &material.result,
        observed_at,
    )?;
    let (deadline, policies) = command.deadline.clone().into_parts();
    let DeadlineChange::Register { definition } = &deadline.change else {
        return Err(invalid("derived.deadline.action"));
    };
    let policies = policies.ok_or_else(|| invalid("derived.tracking"))?;
    let input = &definition.input;
    let FactDeclaration::Known(TriggerSourceRef::HearingResult(reference)) = input.selection.source
    else {
        return Err(invalid("derived.source"));
    };
    let result = &material.result;
    if input.selection.case_id != case
        || reference.hearing_id != command.result.hearing_id
        || reference.result_id != command.result.result_id
        || reference.revision != result.result_revision
        || material.profile.definition.trigger().family() != TriggerFamily::HearingResult
    {
        return Err(invalid("derived.source"));
    }
    deadline_profile_receipt_matches(hasher, &material.profile)?;
    deadline_profile_receipt_matches(hasher, &material.profile_head)?;
    if material.profile.id != definition.profile.id
        || material.profile.revision != definition.profile.revision
    {
        return Err(invalid("derived.profile"));
    }
    validate_profile_selection(
        &material.profile,
        &material.profile_head,
        Some(policies.profile),
    )?;
    let calendar = checked_calendar(
        hasher,
        input.calendar,
        material.calendar.as_ref(),
        material.calendar_head.as_ref(),
    )?;
    if material
        .calendar_head
        .as_ref()
        .is_some_and(|head| head.status != JudicialCalendarStatus::Published)
    {
        return Err(invalid("withdrawn dependency requires explicit review"));
    }
    if let (Some(exact), Some(head)) = (&material.calendar, &material.calendar_head) {
        if policies.calendar == TrackingPolicy::Follow && exact.revision != head.revision {
            return Err(invalid("derived.calendar.head"));
        }
        if exact.revision == head.revision {
            let mut selected_bytes = Vec::new();
            let mut head_bytes = Vec::new();
            evidence::calendar(&mut selected_bytes, exact);
            evidence::calendar(&mut head_bytes, head);
            if selected_bytes != head_bytes {
                return Err(invalid("derived.calendar.capture"));
            }
        }
    }
    let responsible = &material.responsible;
    if responsible.id != definition.responsible
        || responsible.email.trim().is_empty()
        || !responsible.role.allows(Permission::ReadDeadline)
    {
        return Err(DeadlineError::ResponsibleUnavailable.into());
    }
    let trigger = extract_trigger_time(
        material.profile.definition.trigger(),
        &input.selection,
        Some(TriggerMaterial::HearingResult {
            case_id: case,
            hearing_id: command.result.hearing_id,
            result_id: command.result.result_id,
            revision: result.result_revision,
            values: &result.values,
            digests: HearingTriggerDigests {
                values: result.values_digest,
                submission: result.submission_digest,
            },
        }),
    )
    .map_err(|_| invalid("derived.source"))?;
    let evaluation =
        evaluate_extracted_deadline(&material.profile.definition, input, trigger, calendar)
            .map_err(|error| match error {
                DeadlineEvaluationError::Inputs(error) => error,
                DeadlineEvaluationError::Invalid(field) => invalid(field),
            })?;
    let review_digest = canonical::review_digest(hasher, actor, &command, &material, &evaluation)?;
    Ok(HearingDerivedDeadlineDraft {
        command,
        material,
        actor: actor.clone(),
        evaluation,
        review_digest,
    })
}
