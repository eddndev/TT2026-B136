use super::*;
use crate::{
    deadline_inputs::DeadlineSourceDetail,
    deadline_reevaluation::{ObservationRole, Observations},
    deadlines::evidence,
};

pub(crate) fn selected(
    hasher: &dyn DocumentHasher,
    base: &DeadlineDetail,
    inputs: &DeadlineReevaluationInputs,
) -> Result<()> {
    let old = &base.calculation.material;
    let new = &inputs.material;
    let administration = base
        .tracking
        .as_ref()
        .map_or(&old.administration, |tracking| &tracking.administration);
    crate::deadlines::validate_administration_capture(hasher, administration, &new.administration)?;
    let mut previous = Vec::new();
    let mut current = Vec::new();
    evidence::source(&mut previous, hasher, old.source.as_ref());
    evidence::source(&mut current, hasher, new.source.as_ref());
    if let Some(value) = &old.calendar {
        evidence::calendar(&mut previous, value);
    }
    if let Some(value) = &new.calendar {
        evidence::calendar(&mut current, value);
    }
    if old.case_id != new.case_id
        || old.source != new.source
        || old.calendar != new.calendar
        || previous != current
    {
        return Err(inconsistent(
            "technical inputs replaced selected historical evidence",
        ));
    }
    let profile = &base.calculation.profile;
    let head = &inputs.profile_head;
    previous.clear();
    current.clear();
    evidence::profile(&mut previous, profile);
    evidence::profile(&mut current, head);
    if profile.id != head.id
        || profile.definition.scope() != head.definition.scope()
        || profile.revision > head.revision
        || (profile.revision == head.revision && (profile != head || previous != current))
    {
        return Err(inconsistent(
            "technical profile head identity, scope or metadata differs",
        ));
    }
    Ok(())
}

pub(crate) fn advance(old: &Observations, new: &Observations) -> Result<()> {
    for before in &old.entries {
        let after = new
            .entries
            .iter()
            .find(|entry| entry.role == before.role)
            .ok_or_else(|| inconsistent("technical observation disappeared"))?;
        if before.family != after.family
            || before.id != after.id
            || before.case_id != after.case_id
            || before.hearing_id != after.hearing_id
            || before.parent_resolution.map(|value| value.id)
                != after.parent_resolution.map(|value| value.id)
            || before.revision > after.revision
            || (before.revision == after.revision && before != after)
        {
            return Err(inconsistent(
                "technical observations regressed or replaced immutable evidence",
            ));
        }
    }
    Ok(())
}

pub(super) fn cause(
    old: &Observations,
    new: &Observations,
    cause: TechnicalCause,
    inputs: &DeadlineReevaluationInputs,
) -> Result<Option<DeadlineReevaluationNoChange>> {
    let TechnicalCause::SourceEvent { event, .. } = cause else {
        return Ok(None);
    };
    let Some(observed) = new.entries.iter().find(|entry| {
        entry.family == event.family
            && entry.id == event.source_id
            && entry.case_id == event.case_id
            && entry.hearing_id == event.hearing_id
    }) else {
        return Ok(Some(DeadlineReevaluationNoChange::DependencyNotSelected));
    };
    if event.revision > observed.revision {
        return Err(inconsistent(
            "event is newer than its current dependency head",
        ));
    }
    if event.revision == observed.revision
        && head_operation(inputs, observed.role) != Some(event.operation_id)
    {
        return Err(inconsistent("exact head event names a different operation"));
    }
    if old
        .entries
        .iter()
        .find(|entry| entry.role == observed.role)
        .is_some_and(|entry| event.revision <= entry.revision)
    {
        return Ok(Some(DeadlineReevaluationNoChange::AlreadyObserved));
    }
    Ok(None)
}

fn head_operation(
    inputs: &DeadlineReevaluationInputs,
    role: ObservationRole,
) -> Option<domain::typed_participants::Uuid> {
    match role {
        ObservationRole::Profile => Some(inputs.profile_head.receipt.operation_id.as_uuid()),
        ObservationRole::Calendar => inputs
            .material
            .calendar_head
            .as_ref()
            .map(|value| value.receipt.operation_id.as_uuid()),
        ObservationRole::NotificationParent => inputs
            .notification_parent_head
            .as_ref()
            .map(|value| value.snapshot.metadata().receipt.operation_id.as_uuid()),
        ObservationRole::Source => inputs
            .material
            .source_head
            .as_ref()
            .map(|value| match value {
                DeadlineSourceDetail::Fact(value) => {
                    value.snapshot.metadata().receipt.operation_id.as_uuid()
                }
                DeadlineSourceDetail::HearingResult(value) => {
                    value.snapshot.receipt.operation_id.as_uuid()
                }
            }),
    }
}
