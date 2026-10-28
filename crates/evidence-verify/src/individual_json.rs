//! The `individual-signing.json` entry of individual signing packages.
//!
//! The server writes the entry as compact JSON with these members, each byte
//! string as an array of integers 0 to 255:
//!
//! `format_version` (1), `binding_statement` (142 bytes), `binding_signature`
//! (384 bytes), `binding_root_der` (1 to 16384 bytes), `binding_crl_der`
//! (1 to 1048576 bytes), `binding_registered_at` (Unix seconds),
//! `consent_statement` (258 bytes), `consent_signature` (384 bytes) and
//! `accepted_at` (Unix seconds).
//!
//! The exact member set is required in any order, matching what the server
//! reads back. The order of the two instants is checked by the context
//! component, not here. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use crate::json::{self, Limits, Value};
use crate::report::Finding;

#[cfg(test)]
mod tests;

/// Largest admitted entry, the package limit for this entry.
pub const MAX_ENTRY_BYTES: usize = 4_718_592;

const BINDING_STATEMENT_BYTES: usize = 142;
const CONSENT_STATEMENT_BYTES: usize = 258;
const SIGNATURE_BYTES: usize = 384;
const ROOT_MAX_BYTES: usize = 16 * 1024;
const CRL_MAX_BYTES: usize = 1024 * 1024;

/// Covers the largest admitted entry: the byte arrays at their maxima plus
/// the root object and its members, with room to report an oversized array
/// by its length rather than by the value count.
const MAX_VALUES: usize = 1_100_000;

const KEYS: [&str; 9] = [
    "format_version",
    "binding_statement",
    "binding_signature",
    "binding_root_der",
    "binding_crl_der",
    "binding_registered_at",
    "consent_statement",
    "consent_signature",
    "accepted_at",
];

/// The parsed entry; byte fields hold the exact captured bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndividualSigning {
    pub format_version: u8,
    pub binding_statement: Vec<u8>,
    pub binding_signature: Vec<u8>,
    pub binding_root_der: Vec<u8>,
    pub binding_crl_der: Vec<u8>,
    pub binding_registered_at: i64,
    pub consent_statement: Vec<u8>,
    pub consent_signature: Vec<u8>,
    pub accepted_at: i64,
}

/// Parse the entry exactly as the server writes it.
///
/// A size above [`MAX_ENTRY_BYTES`] or a `format_version` other than 1 is
/// not supported; any other deviation is failed.
pub fn parse(bytes: &[u8]) -> Result<IndividualSigning, Finding> {
    if bytes.len() > MAX_ENTRY_BYTES {
        return Err(Finding::not_supported(
            "individual_json.too_large",
            format!("{} bytes", bytes.len()),
        ));
    }
    let value = json::parse(
        bytes,
        Limits {
            max_values: MAX_VALUES,
        },
    )
    .map_err(|error| {
        Finding::failed(
            "individual_json.malformed",
            format!("{} at byte {}", error.code(), error.offset),
        )
    })?;
    let members = value
        .as_object()
        .ok_or_else(|| Finding::failed("individual_json.not_object", ""))?;
    if let Some((key, _)) = members
        .iter()
        .find(|(key, _)| !KEYS.contains(&key.as_str()))
    {
        return Err(Finding::failed("individual_json.unknown_key", key.clone()));
    }
    let member = |key: &'static str| {
        value
            .get(key)
            .ok_or_else(|| Finding::failed("individual_json.missing_key", key))
    };
    for key in KEYS {
        member(key)?;
    }
    let format_version = integer(member("format_version")?, "format_version")?;
    if format_version != 1 {
        return Err(Finding::not_supported(
            "individual_json.format_version",
            format_version.to_string(),
        ));
    }
    let exact = |key, length| byte_field(member(key)?, key, length, length);
    let bounded = |key, maximum| byte_field(member(key)?, key, 1, maximum);
    Ok(IndividualSigning {
        format_version: 1,
        binding_statement: exact("binding_statement", BINDING_STATEMENT_BYTES)?,
        binding_signature: exact("binding_signature", SIGNATURE_BYTES)?,
        binding_root_der: bounded("binding_root_der", ROOT_MAX_BYTES)?,
        binding_crl_der: bounded("binding_crl_der", CRL_MAX_BYTES)?,
        binding_registered_at: instant(member("binding_registered_at")?, "binding_registered_at")?,
        consent_statement: exact("consent_statement", CONSENT_STATEMENT_BYTES)?,
        consent_signature: exact("consent_signature", SIGNATURE_BYTES)?,
        accepted_at: instant(member("accepted_at")?, "accepted_at")?,
    })
}

fn integer(value: &Value, key: &'static str) -> Result<i128, Finding> {
    value
        .as_integer()
        .ok_or_else(|| Finding::failed("individual_json.type", key))
}

/// Earliest instant the server can represent, -9999-01-01T00:00:00Z.
const MIN_INSTANT: i64 = -377_705_116_800;
/// Latest instant the server can represent, 9999-12-31T23:59:59Z.
const MAX_INSTANT: i64 = 253_402_300_799;

/// An instant in the range that `OffsetDateTime::from_unix_timestamp`
/// accepts in `crates/application/src/documents/individual_evidence.rs`,
/// which the server applies before it writes or reads the entry: the years
/// -9999 to 9999, since the workspace does not enable large dates.
fn instant(value: &Value, key: &'static str) -> Result<i64, Finding> {
    i64::try_from(integer(value, key)?)
        .ok()
        .filter(|seconds| (MIN_INSTANT..=MAX_INSTANT).contains(seconds))
        .ok_or_else(|| Finding::failed("individual_json.instant_range", key))
}

fn byte_field(
    value: &Value,
    key: &'static str,
    minimum: usize,
    maximum: usize,
) -> Result<Vec<u8>, Finding> {
    let items = value
        .as_array()
        .ok_or_else(|| Finding::failed("individual_json.type", key))?;
    let mut output = Vec::with_capacity(items.len().min(maximum));
    for item in items {
        let number = integer(item, key)?;
        let byte =
            u8::try_from(number).map_err(|_| Finding::failed("individual_json.byte_range", key))?;
        output.push(byte);
    }
    if output.len() < minimum || output.len() > maximum {
        return Err(Finding::failed(
            "individual_json.length",
            format!("{key}: {} bytes", output.len()),
        ));
    }
    Ok(output)
}
