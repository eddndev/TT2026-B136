//! Exact source binding for declared temporal operands, without legal inference.

mod extract;
mod integrity;
mod material;
mod outcome;
mod selection;

pub use extract::extract_trigger_time;
pub use material::{
    FactTriggerDigests, HearingTriggerDigests, TriggerDigests, TriggerMaterial, TriggerProvenance,
    TriggerSourceSnapshot,
};
pub use outcome::{
    TriggerBlock, TriggerExtraction, TriggerIntegrityError, TriggerOutcome, TriggeredArithmetic,
};
pub use selection::{
    QualifiedTriggerPurpose, QualifiedTriggerTime, TriggerFamily, TriggerField, TriggerRequirement,
    TriggerSelection, TriggerSourceRef,
};

use crate::{
    deadline_arithmetic::{evaluate_deadline_arithmetic, ArithmeticRule},
    judicial_calendars::JudicialCalendarValues,
};

/// Preserve extraction even when it blocks arithmetic; never substitute another time.
///
/// Rules and calendar values are supplied explicitly. See docs/deadline-triggers.md.
pub fn evaluate_triggered_arithmetic(
    requirement: TriggerRequirement,
    selection: &TriggerSelection,
    material: Option<TriggerMaterial<'_>>,
    rule: ArithmeticRule,
    calendar: Option<&JudicialCalendarValues>,
) -> Result<TriggeredArithmetic, TriggerIntegrityError> {
    let trigger = extract_trigger_time(requirement, selection, material)?;
    let arithmetic = match trigger.outcome() {
        TriggerOutcome::Extracted { at } => Some(evaluate_deadline_arithmetic(rule, *at, calendar)),
        TriggerOutcome::Blocked(_) => None,
    };
    Ok(TriggeredArithmetic {
        rule,
        trigger,
        arithmetic,
    })
}
