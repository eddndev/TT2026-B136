//! The acceptance receipt of individual signing v2, `recibo-aceptacion.json`.
//!
//! The server writes it at export from stored data, as RFC 8785 JSON with
//! exactly `profile` (`qadra-acceptance-receipt`), `profile_version` (1),
//! `operation_id`, `manifest_sha256`, `accepted_at_seconds`,
//! `accepted_at_nanoseconds` (below one second) and `provenance`
//! (`application-record`). No time-stamp covers it: it states the
//! application record of the acceptance instant, which the validator
//! compares with `individual-signing.json` and the manifest. Rationale:
//! the section "The evidence manifest" of
//! `docs/adr/0085-in-browser-evidence-validation.md`.

#[cfg(test)]
mod tests;

use crate::json::{self, Limits, Value};
use crate::manifest::{json_finding, schema::Fields};
use crate::report::{hex, Finding};

/// Package entry holding the receipt.
pub const ENTRY: &str = "recibo-aceptacion.json";
/// Value of the `profile` member.
pub const PROFILE: &str = "qadra-acceptance-receipt";
/// Value of the `profile_version` member.
pub const PROFILE_VERSION: u64 = 1;
/// Value of the `provenance` member.
pub const PROVENANCE: &str = "application-record";

/// Code of every rejection of the receipt schema.
const SCHEMA: &str = "receipt.schema";
const KEYS: [&str; 7] = [
    "profile",
    "profile_version",
    "operation_id",
    "manifest_sha256",
    "accepted_at_seconds",
    "accepted_at_nanoseconds",
    "provenance",
];
const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;

/// The parsed receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    /// Lowercase hyphenated UUID of the acceptance operation.
    pub operation_id: String,
    /// SHA-256 of the exact manifest entry bytes.
    pub manifest_sha256: [u8; 32],
    /// Unix seconds of the acceptance, `0..=2^53 - 1`.
    pub accepted_at_seconds: i64,
    /// Nanoseconds within that second, `0..=999_999_999`.
    pub accepted_at_nanoseconds: u32,
}

/// The canonical bytes of `receipt`; fails with the finding [`parse`]
/// would report when the result would not be admitted.
pub fn encode(receipt: &Receipt) -> Result<Vec<u8>, Finding> {
    let text = |value: &str| Value::String(value.to_owned());
    let member = |key: &str, value: Value| (key.to_owned(), value);
    let value = Value::Object(vec![
        member("profile", text(PROFILE)),
        member(
            "profile_version",
            Value::Integer(i128::from(PROFILE_VERSION)),
        ),
        member("operation_id", text(&receipt.operation_id)),
        member("manifest_sha256", text(&hex(&receipt.manifest_sha256))),
        member(
            "accepted_at_seconds",
            Value::Integer(i128::from(receipt.accepted_at_seconds)),
        ),
        member(
            "accepted_at_nanoseconds",
            Value::Integer(i128::from(receipt.accepted_at_nanoseconds)),
        ),
        member("provenance", text(PROVENANCE)),
    ]);
    let bytes = json::canonical(&value).map_err(json_finding)?;
    parse(&bytes)?;
    Ok(bytes)
}

/// Parse the receipt entry `bytes`: canonical bytes with exactly the seven
/// members and their fixed values.
pub fn parse(bytes: &[u8]) -> Result<Receipt, Finding> {
    let value = json::parse_canonical(bytes, Limits { max_values: 16 }).map_err(json_finding)?;
    let fields = Fields::of(&value, &KEYS, "receipt", SCHEMA)?;
    fields.fixed("profile", PROFILE)?;
    if fields.integer("profile_version", 0)? != PROFILE_VERSION {
        return Err(Finding::failed(SCHEMA, "receipt/profile_version is not 1"));
    }
    fields.fixed("provenance", PROVENANCE)?;
    let nanoseconds = u32::try_from(fields.integer("accepted_at_nanoseconds", 0)?)
        .ok()
        .filter(|nanoseconds| *nanoseconds < NANOSECONDS_PER_SECOND)
        .ok_or_else(|| {
            Finding::failed(
                SCHEMA,
                "receipt/accepted_at_nanoseconds is not below one second",
            )
        })?;
    let seconds = i64::try_from(fields.integer("accepted_at_seconds", 0)?)
        .map_err(|_| Finding::failed(SCHEMA, "receipt/accepted_at_seconds is out of range"))?;
    Ok(Receipt {
        operation_id: fields.uuid("operation_id")?,
        manifest_sha256: fields.digest("manifest_sha256")?,
        accepted_at_seconds: seconds,
        accepted_at_nanoseconds: nanoseconds,
    })
}
