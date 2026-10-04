use super::{inconsistent, port, replay};
use application::{hearing_derived_deadlines::HearingDerivedDeadlineRecord, ApplicationError};
use postgres::Transaction;

/// The ordinary result and its source event already belong to this transaction.
/// Insert the deadline and both remaining audit entries before the origin guard.
pub(super) fn insert(
    tx: &mut Transaction<'_>,
    record: &HearingDerivedDeadlineRecord,
) -> Result<(), ApplicationError> {
    let evidence = record.evidence();
    let deadline = &evidence.deadline;
    let result = &evidence.result.snapshot;
    let source_sequence = i64::try_from(evidence.source_event.sequence)
        .map_err(|_| inconsistent("compound source event sequence overflows"))?;
    crate::deadline_postgres::write::insert(tx, deadline, &crate::RingSha256Hasher)?;
    crate::audit_postgres::append_transaction(
        tx,
        &evidence.actor.email,
        "deadline.registered",
        &format!(
            "case:{}:deadline:{}:revision:{}:operation:{}:submission:{}:capture:{}",
            deadline.case_id,
            deadline.id,
            deadline.revision.get(),
            deadline.receipt.operation_id,
            deadline.receipt.submission_digest.to_hex(),
            deadline.receipt.capture_digest.to_hex()
        ),
        deadline.recorded_at,
    )?;
    let audit = crate::audit_postgres::append_transaction(
        tx,
        &evidence.actor.email,
        "hearing_derived_deadline.registered",
        &replay::marker(record),
        deadline.recorded_at,
    )?;
    let audit_sequence = i64::try_from(audit.event.sequence)
        .map_err(|_| inconsistent("compound audit sequence overflows"))?;
    tx.execute(
        "INSERT INTO case_hearing_derived_deadline_origins(
            operation_id,case_id,hearing_id,result_id,result_revision,deadline_id,
            deadline_revision,deadline_operation_id,source_event_sequence,actor_id,
            actor_email,actor_role,review_canonical,review_digest,capture_canonical,
            capture_digest,audit_sequence)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17)",
        &[
            &evidence.command.result.operation_id.as_uuid(),
            &result.case_id.as_uuid(),
            &result.hearing_id.as_uuid(),
            &result.id.as_uuid(),
            &i64::from(result.revision.get()),
            &deadline.id.as_uuid(),
            &i64::from(deadline.revision.get()),
            &deadline.receipt.operation_id.as_uuid(),
            &source_sequence,
            &evidence.actor.id.as_uuid(),
            &evidence.actor.email,
            &evidence.actor.role.as_str(),
            &record.review_bytes(),
            &evidence.review_digest.as_bytes().as_slice(),
            &record.capture_bytes(),
            &evidence.capture_digest.as_bytes().as_slice(),
            &audit_sequence,
        ],
    )
    .map_err(port)?;
    Ok(())
}
