use crate::error::ApiError;
use application::audit_query::AuditEventQuery;
use std::collections::BTreeMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(super) fn parse(raw: Option<&str>) -> Result<AuditEventQuery, ApiError> {
    let raw = raw.ok_or_else(invalid)?;
    if raw.len() > 16_384 {
        return Err(invalid());
    }
    let mut fields = BTreeMap::new();
    for part in raw.split('&') {
        let (key, value) = part.split_once('=').ok_or_else(invalid)?;
        let key = decode(key)?;
        if ![
            "from", "until", "actor", "action", "resource", "limit", "cursor",
        ]
        .contains(&key.as_str())
            || fields.insert(key, decode(value)?).is_some()
        {
            return Err(invalid());
        }
    }
    let from = instant(fields.get("from").ok_or_else(invalid)?)?;
    let until = instant(fields.get("until").ok_or_else(invalid)?)?;
    let limit = match fields.get("limit") {
        None => 20,
        Some(raw) => {
            let value: u32 = raw.parse().map_err(|_| invalid())?;
            if value.to_string() != *raw {
                return Err(invalid());
            }
            value
        }
    };
    let get = |key| fields.get(key).map(String::as_str);
    AuditEventQuery::new(
        from,
        until,
        get("actor"),
        get("action"),
        get("resource"),
        limit,
        get("cursor"),
    )
    .map_err(|_| invalid())
}
fn instant(raw: &str) -> Result<OffsetDateTime, ApiError> {
    let excess_precision = raw
        .split_once('.')
        .is_some_and(|(_, fraction)| fraction.bytes().take_while(u8::is_ascii_digit).count() > 9);
    if raw.len() > 35 || raw.get(17..19) == Some("60") || excess_precision {
        return Err(invalid());
    }
    OffsetDateTime::parse(raw, &Rfc3339).map_err(|_| invalid())
}
fn decode(raw: &str) -> Result<String, ApiError> {
    let mut input = raw.bytes();
    let mut result = Vec::with_capacity(raw.len());
    while let Some(byte) = input.next() {
        result.push(match byte {
            b'+' => b' ',
            b'%' => {
                let high = digit(input.next().ok_or_else(invalid)?)?;
                let low = digit(input.next().ok_or_else(invalid)?)?;
                (high << 4) | low
            }
            other => other,
        });
    }
    String::from_utf8(result).map_err(|_| invalid())
}
fn digit(value: u8) -> Result<u8, ApiError> {
    char::from(value)
        .to_digit(16)
        .map(|value| value as u8)
        .ok_or_else(invalid)
}
fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_audit_query",
        "audit query requires bounded exact filters and a valid UTC interval",
    )
}
