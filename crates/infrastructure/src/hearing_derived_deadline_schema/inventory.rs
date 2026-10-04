use super::{history, incomplete, port};
use application::{hearing_derived_deadlines::HearingDerivedDeadlineRecord, ApplicationError};
use domain::audit::{chain_digest, GENESIS_PREVIOUS};
use domain::crypto::Sha256Digest;
use postgres::{Client, Transaction};

pub(crate) const BOUNDS: &str = "octet_length(actor_email)<=1280 AND octet_length(actor_role)<=32
    AND octet_length(review_canonical) BETWEEN 5 AND 1048576
    AND octet_length(capture_canonical) BETWEEN 457 AND 5808
    AND octet_length(review_digest)=32 AND octet_length(capture_digest)=32";

pub(crate) fn validate(client: &mut Client) -> Result<(), ApplicationError> {
    let mut tx = crate::audit_postgres::begin_audited(client)?;
    let mut after: Option<uuid::Uuid> = None;
    loop {
        let rows = tx
            .query(
                &format!(
                    "SELECT operation_id,({BOUNDS}) AS bounded
            FROM case_hearing_derived_deadline_origins
            WHERE ($1::uuid IS NULL OR operation_id>$1) ORDER BY operation_id LIMIT 64"
                ),
                &[&after],
            )
            .map_err(port)?;
        if rows.is_empty() {
            break;
        }
        for probe in rows {
            if !probe.get::<_, bool>("bounded") {
                return Err(incomplete());
            }
            let operation: uuid::Uuid = probe.get("operation_id");
            let row = tx
                .query_opt(
                    &format!(
                        "SELECT * FROM case_hearing_derived_deadline_origins
                WHERE operation_id=$1 AND ({BOUNDS})"
                    ),
                    &[&operation],
                )
                .map_err(port)?
                .ok_or_else(incomplete)?;
            let record = history::restore(&mut tx, &row)?;
            verify_audit(&mut tx, row.get("audit_sequence"), &record)?;
            after = Some(operation);
        }
    }
    // Every compound marker must have exactly one origin; ordinary result or
    // deadline markers remain independent and need no compound origin.
    let orphan: bool = tx
        .query_one(
            "SELECT EXISTS(SELECT 1 FROM audit_events a
        LEFT JOIN case_hearing_derived_deadline_origins o ON o.audit_sequence=a.sequence
        WHERE a.action='hearing_derived_deadline.registered' AND o.operation_id IS NULL)",
            &[],
        )
        .map_err(port)?
        .get(0);
    if orphan {
        return Err(incomplete());
    }
    tx.commit().map_err(port)
}

pub(crate) fn verify_audit(
    tx: &mut Transaction<'_>,
    sequence: i64,
    record: &HearingDerivedDeadlineRecord,
) -> Result<(), ApplicationError> {
    let row = tx
        .query_opt(
            "SELECT sequence,timestamp,actor,action,resource,chain
        FROM audit_events WHERE sequence=$1 AND octet_length(timestamp)<=64
        AND octet_length(actor)<=1280 AND octet_length(action)<=64
        AND octet_length(resource)<=256 AND octet_length(chain)=32",
            &[&sequence],
        )
        .map_err(port)?
        .ok_or_else(incomplete)?;
    let entry = crate::audit_postgres::decode_event(row)?;
    let e = record.evidence();
    let marker = format!(
        "hrdc1:operation:{}:capture:{}",
        e.command.result.operation_id,
        e.capture_digest.to_hex()
    );
    if entry.event.actor != e.actor.email
        || entry.event.action != "hearing_derived_deadline.registered"
        || entry.event.resource != marker
        || entry.event.timestamp != e.deadline.recorded_at
        || entry.event.timestamp.offset() != e.deadline.recorded_at.offset()
    {
        return Err(incomplete());
    }
    let previous = if sequence == 0 {
        GENESIS_PREVIOUS
    } else {
        let row = tx
            .query_opt(
                "SELECT chain FROM audit_events WHERE sequence=$1
            AND octet_length(chain)=32",
                &[&(sequence - 1)],
            )
            .map_err(port)?
            .ok_or_else(incomplete)?;
        let bytes: &[u8] = row.get(0);
        Sha256Digest::from_array(bytes.try_into().map_err(|_| incomplete())?)
    };
    if chain_digest(&crate::RingSha256Hasher, &previous, &entry.event)? != entry.chain {
        return Err(incomplete());
    }
    Ok(())
}
