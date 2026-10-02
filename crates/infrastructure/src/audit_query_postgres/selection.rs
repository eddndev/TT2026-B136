use super::{invalid, port};
use application::{audit_query::*, ApplicationError};
use postgres::Transaction;

const PREFLIGHT: &str = r#"SELECT sequence,timestamp_seconds,timestamp_nanos,
    octet_length(actor)::bigint+octet_length(action)::bigint+octet_length(resource)::bigint AS text_bytes
    FROM audit_events WHERE sequence <= $1
    AND (timestamp_seconds,timestamp_nanos) >= ($2,$3)
    AND (timestamp_seconds,timestamp_nanos) < ($4,$5)
    AND ($6::text IS NULL OR actor COLLATE "C"=$6 COLLATE "C")
    AND ($7::text IS NULL OR action COLLATE "C"=$7 COLLATE "C")
    AND ($8::text IS NULL OR resource COLLATE "C"=$8 COLLATE "C")
    AND ($9::bigint IS NULL OR (timestamp_seconds,timestamp_nanos,sequence)>($9,$10,$11))
    ORDER BY timestamp_seconds,timestamp_nanos,sequence LIMIT $12"#;

pub(super) fn read(
    tx: &mut Transaction<'_>,
    query: &AuditEventQuery,
    snapshot: u64,
) -> Result<AuditEventBatch, ApplicationError> {
    let snapshot_i64 = i64::try_from(snapshot).map_err(|_| invalid("head overflow"))?;
    let from_seconds = query.from().unix_timestamp();
    let from_nanos = query.from().nanosecond() as i32;
    let until_seconds = query.until().unix_timestamp();
    let until_nanos = query.until().nanosecond() as i32;
    let cursor = query.cursor().map(|cursor| cursor.after);
    let after_seconds = cursor.map(|p| p.seconds);
    let after_nanos = cursor.map(|p| p.nanos as i32);
    let after_sequence = cursor.map(|p| p.sequence as i64);
    let limit = query.limit() as usize;
    let take = i64::from(query.limit()) + 1;
    let rows = tx
        .query(
            PREFLIGHT,
            &[
                &snapshot_i64,
                &from_seconds,
                &from_nanos,
                &until_seconds,
                &until_nanos,
                &query.actor(),
                &query.action(),
                &query.resource(),
                &after_seconds,
                &after_nanos,
                &after_sequence,
                &take,
            ],
        )
        .map_err(port)?;
    let has_more = rows.len() > limit;
    let selected = &rows[..rows.len().min(limit)];
    let mut bytes = 0u64;
    let mut sequences = Vec::with_capacity(selected.len());
    for row in selected {
        let row_bytes = u64::try_from(row.get::<_, i64>("text_bytes"))
            .map_err(|_| invalid("negative text length"))?;
        bytes = bytes
            .checked_add(row_bytes)
            .ok_or(ApplicationError::AuditQueryCapacityExceeded)?;
        if bytes > MAX_AUDIT_PAGE_TEXT_BYTES as u64 {
            return Err(ApplicationError::AuditQueryCapacityExceeded);
        }
        sequences.push(row.get::<_, i64>("sequence"));
    }
    let rows = tx.query("SELECT sequence,timestamp,actor,action,resource,chain,timestamp_seconds,timestamp_nanos
        FROM audit_events WHERE sequence=ANY($1)
        ORDER BY timestamp_seconds,timestamp_nanos,sequence", &[&sequences]).map_err(port)?;
    if rows.len() != selected.len() {
        return Err(invalid("selection changed"));
    }
    let mut events = Vec::with_capacity(rows.len());
    let mut actual_bytes = 0u64;
    for (row, expected) in rows.into_iter().zip(selected) {
        let text: String = row.get("timestamp");
        let seconds: i64 = row.get("timestamp_seconds");
        let nanos: i32 = row.get("timestamp_nanos");
        let event = crate::audit_postgres::decode_event(row)?.event;
        let position = AuditEventPosition::of(&event);
        actual_bytes += (event.actor.len() + event.action.len() + event.resource.len()) as u64;
        if event.timestamp_rfc3339()? != text
            || position.seconds != seconds
            || i64::from(position.nanos) != i64::from(nanos)
            || i64::try_from(event.sequence).ok() != Some(expected.get("sequence"))
            || seconds != expected.get::<_, i64>("timestamp_seconds")
            || nanos != expected.get::<_, i32>("timestamp_nanos")
            || event.sequence > snapshot
            || event.timestamp < query.from()
            || event.timestamp >= query.until()
            || query.actor().is_some_and(|value| value != event.actor)
            || query.action().is_some_and(|value| value != event.action)
            || query
                .resource()
                .is_some_and(|value| value != event.resource)
            || cursor.is_some_and(|after| position <= after)
        {
            return Err(invalid("row disagrees with its exact selection"));
        }
        events.push(event);
    }
    if actual_bytes != bytes || actual_bytes > MAX_AUDIT_PAGE_TEXT_BYTES as u64 {
        return Err(invalid("text length changed"));
    }
    Ok(AuditEventBatch {
        snapshot_max_sequence: Some(snapshot),
        events,
        has_more,
    })
}
