use super::query::{invalid, kind, status};
use crate::error::ApiError;
use application::agenda::{AgendaCursor, AgendaItemKind, AgendaQuery};
use time::OffsetDateTime;
use uuid::Uuid;

pub(super) fn encode(cursor: AgendaCursor, query: AgendaQuery) -> String {
    let rank = match cursor.kind() {
        AgendaItemKind::Hearing => 0,
        AgendaItemKind::Deadline => 1,
    };
    format!(
        "a1:{}:{}:{}:{}:{}:{}:{}:{}",
        query.from().unix_timestamp(),
        query.until().unix_timestamp(),
        kind(query.kind()),
        status(query.hearing_status()),
        cursor.at().unix_timestamp(),
        cursor.at().nanosecond(),
        rank,
        cursor.id()
    )
}
pub(super) fn decode(value: &str, query: AgendaQuery) -> Result<AgendaCursor, ApiError> {
    if value.len() > 512 || !value.is_ascii() {
        return Err(invalid());
    }
    let fields: Vec<_> = value.split(':').collect();
    if fields.len() != 9 {
        return Err(invalid());
    }
    let seconds = fields[5].parse::<i64>().map_err(|_| invalid())?;
    let nanos = fields[6].parse::<u32>().map_err(|_| invalid())?;
    let at = OffsetDateTime::from_unix_timestamp(seconds)
        .map_err(|_| invalid())?
        .replace_nanosecond(nanos)
        .map_err(|_| invalid())?;
    let kind = match fields[7] {
        "0" => AgendaItemKind::Hearing,
        "1" => AgendaItemKind::Deadline,
        _ => return Err(invalid()),
    };
    let id = Uuid::parse_str(fields[8]).map_err(|_| invalid())?;
    let cursor = AgendaCursor::new(at, kind, id).map_err(|_| invalid())?;
    if encode(cursor, query) != value {
        return Err(invalid());
    }
    Ok(cursor)
}
