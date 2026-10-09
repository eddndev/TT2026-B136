use super::storage::port_error;
use application::{identity::Principal, ApplicationError};
use domain::{cases::CaseId, crypto::DocumentId};
use postgres::Transaction;
use time::OffsetDateTime;

pub(super) fn insert(
    tx: &mut Transaction<'_>,
    document: DocumentId,
    case: CaseId,
    actor: &Principal,
    at: OffsetDateTime,
    audit_sequence: u64,
) -> Result<(), ApplicationError> {
    let sequence = i64::try_from(audit_sequence)
        .map_err(|_| ApplicationError::Port("document upload audit sequence overflow".into()))?;
    tx.execute(
        "INSERT INTO document_upload_origins(document_id,case_id,actor_id,actor_email,
            recorded_at_seconds,recorded_at_nanoseconds,audit_sequence)
         VALUES($1,$2,$3,$4,$5,$6,$7)",
        &[
            &document.as_uuid(),
            &case.as_uuid(),
            &actor.id.as_uuid(),
            &actor.email,
            &at.unix_timestamp(),
            &(at.nanosecond() as i32),
            &sequence,
        ],
    )
    .map_err(port_error)?;
    Ok(())
}
