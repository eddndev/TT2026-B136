//! Internal exact-history loading inside an already authorized transaction.
use application::{
    cases::CurrentCaseAdministration,
    deadline_inputs::{
        extract_checked_deadline_inputs, DeadlineCalendarRef, DeadlineInputError,
        DeadlineInputHeads, DeadlineInputMaterial,
    },
    ApplicationError,
};
use domain::{
    cases::CaseId,
    crypto::DocumentHasher,
    deadline_triggers::{TriggerRequirement, TriggerSelection},
    procedural_facts::FactDeclaration,
};
use postgres::Transaction;

/// Resolve immutable selected sources and the exact heads observed with them.
///
/// The caller owns the transaction, must establish current case authorization
/// before calling, and must append its audit before committing or exposing data.
/// This function neither authenticates nor reads current heads. Captured head
/// references do not attest that those revisions were current when first stored.
/// The enclosing persisted evaluation and its receipt establish that observation.
#[doc(hidden)]
pub fn load_captured_material(
    tx: &mut Transaction<'_>,
    requirement: TriggerRequirement,
    selection: &TriggerSelection,
    calendar: Option<DeadlineCalendarRef>,
    administration: &CurrentCaseAdministration,
    captured: &DeadlineInputHeads,
    hasher: &dyn DocumentHasher,
) -> Result<DeadlineInputMaterial, ApplicationError> {
    validate_administration(tx, selection.case_id, administration, hasher)?;
    let (source, source_head) = match (&selection.source, captured.source) {
        (FactDeclaration::Unknown(_), None) => (None, None),
        (FactDeclaration::Known(selected), Some(head)) => (
            Some(crate::deadline_input_postgres::source(
                tx,
                selection.case_id,
                *selected,
                true,
                hasher,
            )?),
            Some(crate::deadline_input_postgres::source(
                tx,
                selection.case_id,
                head,
                true,
                hasher,
            )?),
        ),
        _ => return Err(inconsistent("captured source head presence differs")),
    };
    let (exact_calendar, calendar_head) = match (calendar, captured.calendar) {
        (None, None) => (None, None),
        (Some(selected), Some(head)) => (
            Some(crate::judicial_calendar_postgres::storage::detail(
                tx,
                selected.id,
                Some(selected.revision),
                hasher,
            )?),
            Some(crate::judicial_calendar_postgres::storage::detail(
                tx,
                head.id,
                Some(head.revision),
                hasher,
            )?),
        ),
        _ => return Err(inconsistent("captured calendar head presence differs")),
    };
    let material = DeadlineInputMaterial {
        case_id: selection.case_id,
        administration: administration.clone(),
        source,
        source_head,
        calendar: exact_calendar,
        calendar_head,
    };
    // Includes the notification head's own exact parent and absence of an
    // agreement selection on hearing heads, independently from selected sources.
    if DeadlineInputHeads::capture(&material) != *captured {
        return Err(inconsistent(
            "captured head differs from its exact revision",
        ));
    }
    extract_checked_deadline_inputs(hasher, requirement, selection, calendar, &material)?;
    Ok(material)
}
fn validate_administration(
    tx: &mut Transaction<'_>,
    case_id: CaseId,
    captured: &CurrentCaseAdministration,
    hasher: &dyn DocumentHasher,
) -> Result<(), ApplicationError> {
    match captured {
        CurrentCaseAdministration::Recorded(snapshot) => {
            let stored =
                crate::hearing_postgres::administration(tx, case_id, snapshot.revision, hasher)?;
            if stored != **snapshot || stored.changed_at.offset() != snapshot.changed_at.offset() {
                return Err(inconsistent(
                    "captured administration differs from its revision",
                ));
            }
        }
        CurrentCaseAdministration::Unrevised(metadata) => {
            let baseline = tx
                .query_opt(
                    "SELECT title,reference,required_initial_revision FROM cases WHERE id=$1
                 AND octet_length(title)<=800 AND octet_length(reference)<=400",
                    &[&case_id.as_uuid()],
                )
                .map_err(port)?
                .ok_or_else(|| inconsistent("original case baseline absent or unbounded"))?;
            if baseline
                .try_get::<_, Option<i64>>("required_initial_revision")
                .map_err(inconsistent)?
                .is_some()
                || baseline
                    .try_get::<_, String>("title")
                    .map_err(inconsistent)?
                    != metadata.title()
                || baseline
                    .try_get::<_, String>("reference")
                    .map_err(inconsistent)?
                    != metadata.reference()
            {
                return Err(inconsistent(
                    "captured unrevised baseline differs from original case",
                ));
            }
        }
    }
    Ok(())
}
fn inconsistent(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineInputError::Inconsistent(error.to_string()).into()
}
fn port(error: postgres::Error) -> ApplicationError {
    ApplicationError::Port(format!("deadline input history database: {error}"))
}
