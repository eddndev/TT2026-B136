//! Decoding of the policy JSON the Web Worker sends.
//!
//! The document is
//! `{"pinned": [{"fingerprint", "purposes", "label"}], "served": null |
//! {"roots": [{"der_base64", "purposes"}], "crls_base64": [...]},
//! "admitted_policies": [...], "current_instant": <unix seconds>}`.
//! `admitted_policies` is optional and defaults to the internal authority
//! policy; `served` may be omitted or null when the trust route was
//! unavailable. Fingerprints are 64 lowercase hexadecimal digits so that a
//! pinned value has exactly one spelling.

use base64::{engine::general_purpose::STANDARD, Engine};
use evidence_verify::policy::{
    PinnedRoot, Policy, Purpose, Purposes, ServedRoot, ServedTrust, DEFAULT_ADMITTED_POLICY,
};
use serde_json::{Map, Value};

/// Why a policy document was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    /// Not JSON, or a field is missing or has the wrong type.
    Malformed,
    /// A pinned fingerprint is not 64 lowercase hexadecimal digits.
    Fingerprint,
    /// A purpose list is empty or names an unknown purpose.
    Purpose,
    /// A root or CRL is not standard padded base64.
    Base64,
}

impl PolicyError {
    /// The stable code the Web Worker maps to a message.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed => "policy.malformed",
            Self::Fingerprint => "policy.fingerprint",
            Self::Purpose => "policy.purpose",
            Self::Base64 => "policy.base64",
        }
    }
}

/// Parse the policy document `text`.
pub fn parse_policy(text: &str) -> Result<Policy, PolicyError> {
    let value: Value = serde_json::from_str(text).map_err(|_| PolicyError::Malformed)?;
    let object = value.as_object().ok_or(PolicyError::Malformed)?;
    let pinned = array(object, "pinned")?
        .iter()
        .map(pinned_root)
        .collect::<Result<Vec<_>, _>>()?;
    let served = match object.get("served") {
        None | Some(Value::Null) => None,
        Some(Value::Object(served)) => Some(served_trust(served)?),
        Some(_) => return Err(PolicyError::Malformed),
    };
    let admitted_policies = match object.get("admitted_policies") {
        None => vec![DEFAULT_ADMITTED_POLICY.to_owned()],
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| text_value(item).map(str::to_owned))
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(PolicyError::Malformed),
    };
    let current_instant = object
        .get("current_instant")
        .and_then(Value::as_i64)
        .ok_or(PolicyError::Malformed)?;
    Ok(Policy {
        pinned,
        served,
        admitted_policies,
        current_instant,
    })
}

fn array<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a Vec<Value>, PolicyError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or(PolicyError::Malformed)
}

fn field<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, PolicyError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(PolicyError::Malformed)
}

fn text_value(value: &Value) -> Result<&str, PolicyError> {
    value.as_str().ok_or(PolicyError::Malformed)
}

fn pinned_root(value: &Value) -> Result<PinnedRoot, PolicyError> {
    let object = value.as_object().ok_or(PolicyError::Malformed)?;
    let sha256 = fingerprint(field(object, "fingerprint")?)?;
    let purposes = purposes(array(object, "purposes")?)?;
    let label = field(object, "label")?;
    if label.is_empty() {
        return Err(PolicyError::Malformed);
    }
    Ok(PinnedRoot {
        sha256,
        purposes,
        label: label.to_owned(),
    })
}

fn served_trust(object: &Map<String, Value>) -> Result<ServedTrust, PolicyError> {
    let mut roots = Vec::new();
    for value in array(object, "roots")? {
        let root = value.as_object().ok_or(PolicyError::Malformed)?;
        let der = decode(field(root, "der_base64")?)?;
        let purposes = purposes(array(root, "purposes")?)?;
        roots.push(ServedRoot { der, purposes });
    }
    let crls = array(object, "crls_base64")?
        .iter()
        .map(|value| decode(text_value(value)?))
        .collect::<Result<_, _>>()?;
    Ok(ServedTrust { roots, crls })
}

fn decode(text: &str) -> Result<Vec<u8>, PolicyError> {
    STANDARD.decode(text).map_err(|_| PolicyError::Base64)
}

fn purposes(items: &[Value]) -> Result<Purposes, PolicyError> {
    if items.is_empty() {
        return Err(PolicyError::Purpose);
    }
    let mut set = Purposes::default();
    for item in items {
        let name = text_value(item)?;
        let purpose = Purpose::ALL
            .into_iter()
            .find(|purpose| purpose.name() == name)
            .ok_or(PolicyError::Purpose)?;
        match purpose {
            Purpose::ServerSeal => set.server_seal = true,
            Purpose::IndividualSigning => set.individual_signing = true,
            Purpose::TimeStamping => set.time_stamping = true,
        }
    }
    Ok(set)
}

fn fingerprint(text: &str) -> Result<[u8; 32], PolicyError> {
    let digits = text.as_bytes();
    let lowercase_hex = |byte: &u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte);
    if digits.len() != 64 || !digits.iter().all(lowercase_hex) {
        return Err(PolicyError::Fingerprint);
    }
    let mut out = [0u8; 32];
    for (byte, pair) in out.iter_mut().zip(digits.chunks(2)) {
        *byte = (nibble(pair[0]) << 4) | nibble(pair[1]);
    }
    Ok(out)
}

fn nibble(digit: u8) -> u8 {
    if digit.is_ascii_digit() {
        digit - b'0'
    } else {
        digit - b'a' + 10
    }
}
