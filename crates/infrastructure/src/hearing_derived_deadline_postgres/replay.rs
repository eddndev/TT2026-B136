use super::{inconsistent, port};
use crate::hearing_derived_deadline_schema::{history, inventory};
use application::{
    deadlines::DeadlineError,
    hearing_derived_deadlines::{
        restore_hearing_derived_deadline, HearingDerivedDeadlineCommand,
        HearingDerivedDeadlineRecord,
    },
    identity::Principal,
    ApplicationError,
};
use domain::cases::CaseId;
use postgres::Transaction;

/// The caller authorizes the current actor and case in this audited transaction.
/// Only the immutable origin can confirm a compound operation's prior success.
pub(super) fn load(
    tx: &mut Transaction<'_>,
    principal: &Principal,
    case: CaseId,
    command: &HearingDerivedDeadlineCommand,
) -> Result<Option<HearingDerivedDeadlineRecord>, ApplicationError> {
    let operation = command.result.operation_id.as_uuid();
    let Some(probe) = tx
        .query_opt(
            &format!(
                "SELECT case_id,hearing_id,result_id,result_revision,deadline_id,
                deadline_revision,deadline_operation_id,actor_id,({}) AS bounded
                FROM case_hearing_derived_deadline_origins WHERE operation_id=$1",
                inventory::BOUNDS
            ),
            &[&operation],
        )
        .map_err(port)?
    else {
        return Ok(None);
    };
    let deadline = command.deadline.clone().into_parts().0;
    if probe.get::<_, uuid::Uuid>("case_id") != case.as_uuid()
        || probe.get::<_, uuid::Uuid>("hearing_id") != command.result.hearing_id.as_uuid()
        || probe.get::<_, uuid::Uuid>("result_id") != command.result.result_id.as_uuid()
        || probe.get::<_, uuid::Uuid>("deadline_id") != deadline.deadline_id.as_uuid()
        || probe.get::<_, uuid::Uuid>("deadline_operation_id") != deadline.operation_id.as_uuid()
        || probe.get::<_, uuid::Uuid>("actor_id") != principal.id.as_uuid()
    {
        return Err(DeadlineError::OperationConflict.into());
    }
    if !probe.get::<_, bool>("bounded")
        || probe.get::<_, i64>("result_revision") != 1
        || probe.get::<_, i64>("deadline_revision") != 1
    {
        return Err(inconsistent(
            "compound origin has invalid bounds or revisions",
        ));
    }
    let row = tx
        .query_opt(
            &format!(
                "SELECT * FROM case_hearing_derived_deadline_origins
                WHERE operation_id=$1 AND case_id=$2 AND ({})",
                inventory::BOUNDS
            ),
            &[&operation, &case.as_uuid()],
        )
        .map_err(port)?
        .ok_or_else(|| inconsistent("compound origin changed during replay"))?;
    let record = history::restore(tx, &row)?;
    inventory::verify_audit(tx, row.get("audit_sequence"), &record)?;

    // Re-encoding binds the submitted command, including declared offsets, to
    // the original review. Historical actor fields remain the recorded values.
    let mut requested = record.evidence().clone();
    requested.command = command.clone();
    restore_hearing_derived_deadline(&crate::RingSha256Hasher, requested)?;
    Ok(Some(record))
}

pub(super) fn marker(record: &HearingDerivedDeadlineRecord) -> String {
    let evidence = record.evidence();
    format!(
        "hrdc1:operation:{}:capture:{}",
        evidence.command.result.operation_id,
        evidence.capture_digest.to_hex()
    )
}
