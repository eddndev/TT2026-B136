use super::*;
use crate::{
    cases::case_administration_digest, judicial_calendars::judicial_calendar_receipt_matches,
};
use domain::{
    crypto::DocumentHasher,
    deadline_triggers::{
        evaluate_triggered_arithmetic, extract_trigger_time, TriggerExtraction, TriggerMaterial,
        TriggerRequirement, TriggerSelection, TriggeredArithmetic,
    },
    judicial_calendars::JudicialCalendarValues,
};

/// Self-contained checked extraction; it does not establish access or persisted provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedDeadlineTrigger<'a> {
    extraction: TriggerExtraction,
    calendar: Option<&'a JudicialCalendarValues>,
}
impl<'a> CheckedDeadlineTrigger<'a> {
    pub const fn extraction(&self) -> &TriggerExtraction {
        &self.extraction
    }
    pub const fn calendar(&self) -> Option<&'a JudicialCalendarValues> {
        self.calendar
    }
}

/// Verify all supplied material before extracting the explicitly required field or qualification.
pub fn extract_checked_deadline_inputs<'a>(
    hasher: &dyn DocumentHasher,
    requirement: TriggerRequirement,
    selection: &TriggerSelection,
    calendar: Option<DeadlineCalendarRef>,
    material: &'a DeadlineInputMaterial,
) -> Result<CheckedDeadlineTrigger<'a>, ApplicationError> {
    let checked = checked_material(hasher, selection, calendar, material)?;
    let extraction =
        extract_trigger_time(requirement, selection, checked.source).map_err(inconsistent)?;
    Ok(CheckedDeadlineTrigger {
        extraction,
        calendar: checked.calendar,
    })
}

/// Check self-contained consistency; only a store can establish persisted provenance and access.
pub fn check_deadline_inputs(
    hasher: &dyn DocumentHasher,
    request: &DeadlineInputRequest,
    material: &DeadlineInputMaterial,
) -> Result<TriggeredArithmetic, ApplicationError> {
    let checked = checked_material(hasher, &request.trigger, request.calendar, material)?;
    evaluate_triggered_arithmetic(
        request.requirement,
        &request.trigger,
        checked.source,
        request.rule,
        checked.calendar,
    )
    .map_err(inconsistent)
}

struct CheckedMaterial<'a> {
    source: Option<TriggerMaterial<'a>>,
    calendar: Option<&'a JudicialCalendarValues>,
}
fn checked_material<'a>(
    hasher: &dyn DocumentHasher,
    selection: &TriggerSelection,
    calendar: Option<DeadlineCalendarRef>,
    material: &'a DeadlineInputMaterial,
) -> Result<CheckedMaterial<'a>, ApplicationError> {
    if material.case_id != selection.case_id {
        return Err(inconsistent("material belongs to another case"));
    }
    if let Some(snapshot) = material.administration.snapshot() {
        if snapshot.case_id != material.case_id
            || case_administration_digest(hasher, &snapshot.values) != snapshot.values_digest
        {
            return Err(inconsistent("administration case or values digest differs"));
        }
    }
    let source = super::sources::checked_source(hasher, selection, material)?;
    let calendar = checked_calendar(hasher, calendar, material)?;
    Ok(CheckedMaterial { source, calendar })
}
fn checked_calendar<'a>(
    hasher: &dyn DocumentHasher,
    selected: Option<DeadlineCalendarRef>,
    material: &'a DeadlineInputMaterial,
) -> Result<Option<&'a JudicialCalendarValues>, ApplicationError> {
    let (selected, exact, head) = match (selected, &material.calendar, &material.calendar_head) {
        (None, None, None) => return Ok(None),
        (Some(selected), Some(exact), Some(head)) => (selected, exact, head),
        _ => {
            return Err(inconsistent(
                "calendar and head presence differ from selection",
            ))
        }
    };
    judicial_calendar_receipt_matches(hasher, exact).map_err(inconsistent)?;
    judicial_calendar_receipt_matches(hasher, head).map_err(inconsistent)?;
    if exact.id != selected.id
        || exact.revision != selected.revision
        || head.id != exact.id
        || head.revision < exact.revision
        || head.values.scope() != exact.values.scope()
        || (head.revision == exact.revision && head != exact)
    {
        return Err(inconsistent(
            "calendar identity, revision, scope or exact contents disagree",
        ));
    }
    Ok(Some(&exact.values))
}
