use super::*;
use crate::{
    cases::case_administration_digest, judicial_calendars::judicial_calendar_receipt_matches,
};
use domain::{
    crypto::DocumentHasher,
    deadline_triggers::{evaluate_triggered_arithmetic, TriggeredArithmetic},
    judicial_calendars::JudicialCalendarValues,
};

/// Check self-contained consistency; only a store can establish persisted provenance and access.
pub fn check_deadline_inputs(
    hasher: &dyn DocumentHasher,
    request: &DeadlineInputRequest,
    material: &DeadlineInputMaterial,
) -> Result<TriggeredArithmetic, ApplicationError> {
    if material.case_id != request.trigger.case_id {
        return Err(inconsistent("material belongs to another case"));
    }
    if let Some(snapshot) = material.administration.snapshot() {
        if snapshot.case_id != material.case_id
            || case_administration_digest(hasher, &snapshot.values) != snapshot.values_digest
        {
            return Err(inconsistent("administration case or values digest differs"));
        }
    }
    let source = super::sources::checked_source(hasher, request, material)?;
    let calendar = checked_calendar(hasher, request, material)?;
    evaluate_triggered_arithmetic(
        request.requirement,
        &request.trigger,
        source,
        request.rule,
        calendar,
    )
    .map_err(inconsistent)
}
fn checked_calendar<'a>(
    hasher: &dyn DocumentHasher,
    request: &DeadlineInputRequest,
    material: &'a DeadlineInputMaterial,
) -> Result<Option<&'a JudicialCalendarValues>, ApplicationError> {
    let (selected, exact, head) = match (
        &request.calendar,
        &material.calendar,
        &material.calendar_head,
    ) {
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
