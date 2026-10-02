use super::{query::invalid, AuditEventCursor, AuditEventPosition, AuditEventQuery};
use crate::ApplicationError;
use domain::clock::OffsetDateTime;

pub(super) fn validate(
    query: &AuditEventQuery,
    cursor: AuditEventCursor,
) -> Result<(), ApplicationError> {
    let position = cursor.after;
    if cursor.snapshot_max_sequence > i64::MAX as u64
        || position.sequence > cursor.snapshot_max_sequence
    {
        return Err(invalid("audit cursor sequence exceeds its snapshot"));
    }
    let instant = OffsetDateTime::from_unix_timestamp(position.seconds)
        .and_then(|value| value.replace_nanosecond(position.nanos))
        .map_err(|_| invalid("audit cursor timestamp is invalid"))?;
    if instant < query.from() || instant >= query.until() {
        return Err(invalid("audit cursor timestamp differs from the interval"));
    }
    Ok(())
}

pub(super) fn decode(
    query: &AuditEventQuery,
    raw: &str,
) -> Result<AuditEventCursor, ApplicationError> {
    if raw.len() > 4096 || !raw.is_ascii() {
        return Err(invalid("audit cursor is not bounded ASCII"));
    }
    let parts: Vec<_> = raw.split(':').collect();
    if parts.len() != 12 || parts[0] != "aq1" {
        return Err(invalid("audit cursor structure is invalid"));
    }
    let cursor = AuditEventCursor {
        snapshot_max_sequence: number(parts[1])?,
        after: AuditEventPosition {
            seconds: number(parts[2])?,
            nanos: number(parts[3])?,
            sequence: number(parts[4])?,
        },
    };
    validate(query, cursor)?;
    if encode(query, cursor) != raw {
        return Err(invalid(
            "audit cursor is noncanonical or differs from its filters",
        ));
    }
    Ok(cursor)
}

fn number<T: std::str::FromStr>(value: &str) -> Result<T, ApplicationError> {
    value
        .parse()
        .map_err(|_| invalid("audit cursor number is invalid"))
}

pub(super) fn encode(query: &AuditEventQuery, cursor: AuditEventCursor) -> String {
    format!(
        "aq1:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
        cursor.snapshot_max_sequence,
        cursor.after.seconds,
        cursor.after.nanos,
        cursor.after.sequence,
        query.from().unix_timestamp(),
        query.from().nanosecond(),
        query.until().unix_timestamp(),
        query.until().nanosecond(),
        hex(query.actor()),
        hex(query.action()),
        hex(query.resource()),
    )
}

fn hex(value: Option<&str>) -> String {
    let Some(value) = value else {
        return "-".into();
    };
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.bytes() {
        encoded.push(char::from(DIGITS[usize::from(byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(byte & 15)]));
    }
    encoded
}
