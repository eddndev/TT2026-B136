//! The application records of bilateral contract signing:
//! `contrato-recibo.json` in a full package and `contrato-acuse-parcial.json`
//! in a partial one.
//!
//! The server writes both at export from stored data, as RFC 8785 JSON with
//! exactly their members and `provenance` `application-record`. No
//! time-stamp covers them and they decide no result: the validator parses
//! them for their schema and reports their instants as application records.
//! Rationale: the section "Evidence packages" of
//! `docs/adr/0086-bilateral-contract-signing.md`.

#[cfg(test)]
mod tests;

use super::PROVENANCE;
use crate::json::{self, Limits, Value};
use crate::manifest::{json_finding, schema::Fields};
use crate::report::{hex, Finding};

/// Package entry holding the receipt of a full package.
pub const ENTRY: &str = "contrato-recibo.json";
/// Package entry holding the acknowledgement of a partial package.
pub const PARTIAL_ENTRY: &str = "contrato-acuse-parcial.json";

const RECEIPT_SCHEMA: &str = "receipt.contract_schema";
const PARTIAL_SCHEMA: &str = "receipt.contract_partial_schema";
const RECEIPT_KEYS: [&str; 7] = [
    "provenance",
    "request_id",
    "request_sha256",
    "ceremony_reference",
    "client_accepted_at",
    "firm_accepted_at",
    "completed_at",
];
const PARTIAL_KEYS: [&str; 7] = [
    "provenance",
    "request_id",
    "request_sha256",
    "binding_registered_at",
    "accepted_at",
    "ceremony_reference",
    "exported_at",
];
/// Most characters of the declared fingerprint sheet reference.
const MAX_CEREMONY_REFERENCE: usize = 80;

/// The receipt of a completed request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractReceipt {
    /// Lowercase hyphenated UUID of the request.
    pub request_id: String,
    /// SHA-256 of the 240 request bytes.
    pub request_sha256: [u8; 32],
    /// The declared fingerprint sheet reference, 1 to 80 characters.
    pub ceremony_reference: String,
    /// Unix seconds of each record, `0..=2^53 - 1`.
    pub client_accepted_at: i64,
    pub firm_accepted_at: i64,
    pub completed_at: i64,
}

/// The acknowledgement the Client keeps once position 1 has signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartialReceipt {
    pub request_id: String,
    pub request_sha256: [u8; 32],
    /// Unix seconds of each record, `0..=2^53 - 1`.
    pub binding_registered_at: i64,
    pub accepted_at: i64,
    pub ceremony_reference: String,
    pub exported_at: i64,
}

/// The canonical bytes of `receipt`; fails with the finding
/// [`parse_receipt`] would report when the result would not be admitted.
pub fn encode_receipt(receipt: &ContractReceipt) -> Result<Vec<u8>, Finding> {
    let bytes = canonical(&[
        ("request_id", text(&receipt.request_id)),
        ("request_sha256", text(&hex(&receipt.request_sha256))),
        ("ceremony_reference", text(&receipt.ceremony_reference)),
        ("client_accepted_at", integer(receipt.client_accepted_at)),
        ("firm_accepted_at", integer(receipt.firm_accepted_at)),
        ("completed_at", integer(receipt.completed_at)),
    ])?;
    parse_receipt(&bytes)?;
    Ok(bytes)
}

/// The canonical bytes of `receipt`; fails with the finding
/// [`parse_partial`] would report when the result would not be admitted.
pub fn encode_partial(receipt: &PartialReceipt) -> Result<Vec<u8>, Finding> {
    let bytes = canonical(&[
        ("request_id", text(&receipt.request_id)),
        ("request_sha256", text(&hex(&receipt.request_sha256))),
        (
            "binding_registered_at",
            integer(receipt.binding_registered_at),
        ),
        ("accepted_at", integer(receipt.accepted_at)),
        ("ceremony_reference", text(&receipt.ceremony_reference)),
        ("exported_at", integer(receipt.exported_at)),
    ])?;
    parse_partial(&bytes)?;
    Ok(bytes)
}

/// Parse `contrato-recibo.json`: canonical bytes with exactly its members.
pub fn parse_receipt(bytes: &[u8]) -> Result<ContractReceipt, Finding> {
    let value = parse_value(bytes)?;
    let record = Record::of(&value, &RECEIPT_KEYS, "receipt", RECEIPT_SCHEMA)?;
    Ok(ContractReceipt {
        request_id: record.fields.uuid("request_id")?,
        request_sha256: record.fields.digest("request_sha256")?,
        ceremony_reference: record.reference()?,
        client_accepted_at: record.seconds("client_accepted_at")?,
        firm_accepted_at: record.seconds("firm_accepted_at")?,
        completed_at: record.seconds("completed_at")?,
    })
}

/// Parse `contrato-acuse-parcial.json`: canonical bytes with exactly its
/// members.
pub fn parse_partial(bytes: &[u8]) -> Result<PartialReceipt, Finding> {
    let value = parse_value(bytes)?;
    let record = Record::of(&value, &PARTIAL_KEYS, "acknowledgement", PARTIAL_SCHEMA)?;
    Ok(PartialReceipt {
        request_id: record.fields.uuid("request_id")?,
        request_sha256: record.fields.digest("request_sha256")?,
        binding_registered_at: record.seconds("binding_registered_at")?,
        accepted_at: record.seconds("accepted_at")?,
        ceremony_reference: record.reference()?,
        exported_at: record.seconds("exported_at")?,
    })
}

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn integer(value: i64) -> Value {
    Value::Integer(i128::from(value))
}

/// The canonical bytes of `members` plus the fixed provenance.
fn canonical(members: &[(&str, Value)]) -> Result<Vec<u8>, Finding> {
    let mut object: Vec<(String, Value)> = members
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect();
    object.push(("provenance".to_owned(), text(PROVENANCE)));
    json::canonical(&Value::Object(object)).map_err(json_finding)
}

/// The value of canonical bytes; both records hold fewer than 16 values.
fn parse_value(bytes: &[u8]) -> Result<Value, Finding> {
    json::parse_canonical(bytes, Limits { max_values: 16 }).map_err(json_finding)
}

/// A parsed record with exactly its members and the fixed provenance.
struct Record<'a> {
    fields: Fields<'a>,
    path: &'static str,
    code: &'static str,
}

impl<'a> Record<'a> {
    fn of(
        value: &'a Value,
        keys: &[&str],
        path: &'static str,
        code: &'static str,
    ) -> Result<Self, Finding> {
        let fields = Fields::of(value, keys, path, code)?;
        fields.fixed("provenance", PROVENANCE)?;
        Ok(Record { fields, path, code })
    }

    fn seconds(&self, key: &str) -> Result<i64, Finding> {
        i64::try_from(self.fields.integer(key, 0)?)
            .map_err(|_| Finding::failed(self.code, format!("{}/{key} is out of range", self.path)))
    }

    fn reference(&self) -> Result<String, Finding> {
        let reference = self.fields.text("ceremony_reference")?;
        let length = reference.chars().count();
        if (1..=MAX_CEREMONY_REFERENCE).contains(&length) {
            Ok(reference.to_owned())
        } else {
            Err(Finding::failed(
                self.code,
                format!("{}/ceremony_reference is not 1 to 80 characters", self.path),
            ))
        }
    }
}
