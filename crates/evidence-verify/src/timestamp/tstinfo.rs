//! The `TSTInfo` content of a token.

use der::asn1::ObjectIdentifier;
use der::{Decode, Encode};
use x509_cert::ext::pkix::name::GeneralName;
use x509_tsp::{Accuracy, TstInfo};

use super::malformed;
use super::signer::ID_SHA256;
use super::structure::{der, INTEGER, SEQUENCE};
use crate::der_walk;
use crate::report::Finding;

/// Largest sub-second accuracy field value (RFC 3161, `Accuracy`).
const MAX_SUBSECOND: i16 = 999;
/// Most `TSTInfo` fields: ten in RFC 3161.
const MAX_FIELDS: usize = 10;
/// Index of `genTime` among the `TSTInfo` fields, which are all mandatory
/// up to it.
const GEN_TIME_FIELD: usize = 4;
/// Identifier octet of a GeneralizedTime.
const GENERALIZED_TIME: u8 = 0x18;
/// Digits of `YYYYMMDDHHMMSS`, the whole-second part of a GeneralizedTime.
const WHOLE_SECOND_DIGITS: usize = 14;

/// What `TSTInfo` states. The policy and the imprint are judged by the
/// caller; the other facts are reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TstFacts {
    pub gen_time: i64,
    pub accuracy_seconds: i64,
    pub policy: ObjectIdentifier,
    pub serial: Vec<u8>,
    pub nonce: Option<Vec<u8>>,
    pub ordering: bool,
    pub tsa: Option<String>,
    imprint_algorithm: ObjectIdentifier,
    imprint_plain: bool,
    hashed_message: Vec<u8>,
}

impl TstFacts {
    /// The imprint must be SHA-256 with absent or NULL parameters over
    /// `covered`; anything else is a failed `token.imprint_mismatch`.
    pub(super) fn check_imprint(&self, covered: &[u8; 32]) -> Result<(), Finding> {
        if self.imprint_algorithm != ID_SHA256 || !self.imprint_plain {
            return Err(Finding::failed(
                "token.imprint_mismatch",
                format!("imprint algorithm {}", self.imprint_algorithm),
            ));
        }
        if self.hashed_message != covered {
            return Err(Finding::failed(
                "token.imprint_mismatch",
                "the token covers another digest",
            ));
        }
        Ok(())
    }
}

/// Read the DER `TSTInfo` in `econtent`.
///
/// It must be strict DER. A version other than 1, a `genTime` with
/// fractional seconds or a critical extension is not supported. `tsa`,
/// `ordering` and non-critical extensions are not interpreted.
pub(super) fn parse(econtent: &[u8]) -> Result<TstFacts, Finding> {
    let outer = der_walk::walk(econtent, der_walk::MAX_DEPTH).map_err(der)?;
    if outer.tag != SEQUENCE {
        return Err(malformed("TSTInfo is not a SEQUENCE"));
    }
    let fields = der_walk::children(outer.value, MAX_FIELDS).map_err(der)?;
    let version = fields
        .first()
        .filter(|field| field.tag == INTEGER)
        .ok_or_else(|| malformed("TSTInfo has no version"))?;
    if version.value != [1] {
        return Err(Finding::not_supported(
            "token.version",
            "TSTInfo version is not 1",
        ));
    }
    if let Some(gen_time) = fields.get(GEN_TIME_FIELD) {
        if gen_time.tag == GENERALIZED_TIME && has_der_fraction(gen_time.value) {
            return Err(Finding::not_supported(
                "token.gen_time_fraction",
                "genTime has fractional seconds",
            ));
        }
    }
    let info = TstInfo::from_der(econtent).map_err(|_| malformed("invalid TSTInfo"))?;
    if info.to_der().ok().as_deref() != Some(econtent) {
        return Err(malformed("TSTInfo is not in canonical DER"));
    }
    let extensions = info.extensions.as_deref().unwrap_or_default();
    if let Some(critical) = extensions.iter().find(|extension| extension.critical) {
        return Err(Finding::not_supported(
            "token.critical_extension",
            format!("critical TSTInfo extension {}", critical.extn_id),
        ));
    }
    let gen_time = i64::try_from(info.gen_time.to_unix_duration().as_secs())
        .map_err(|_| malformed("genTime out of range"))?;
    let algorithm = &info.message_imprint.hash_algorithm;
    Ok(TstFacts {
        gen_time,
        accuracy_seconds: accuracy_seconds(info.accuracy.as_ref())?,
        policy: info.policy,
        serial: info.serial_number.as_bytes().to_vec(),
        nonce: info.nonce.map(|nonce| nonce.as_bytes().to_vec()),
        ordering: info.ordering,
        tsa: info.tsa.as_ref().map(tsa_name),
        imprint_algorithm: algorithm.oid,
        imprint_plain: algorithm
            .parameters
            .as_ref()
            .is_none_or(|parameters| parameters.is_null()),
        hashed_message: info.message_imprint.hashed_message.as_bytes().to_vec(),
    })
}

/// Whether `value` is a GeneralizedTime in the DER form with fractional
/// seconds: fourteen digits, a full stop, digits without a trailing zero
/// and `Z`. A value that is not, with or without a fraction, is left to the
/// strict decoder.
fn has_der_fraction(value: &[u8]) -> bool {
    let Some((whole, rest)) = value.split_at_checked(WHOLE_SECOND_DIGITS) else {
        return false;
    };
    let Some(fraction) = rest
        .strip_prefix(b".")
        .and_then(|rest| rest.strip_suffix(b"Z"))
    else {
        return false;
    };
    whole.iter().all(u8::is_ascii_digit)
        && !fraction.is_empty()
        && fraction.iter().all(u8::is_ascii_digit)
        && fraction.last() != Some(&b'0')
}

/// A readable form of the `tsa` name, which is reported and never relied on.
fn tsa_name(name: &GeneralName) -> String {
    match name {
        GeneralName::DirectoryName(name) => name.to_string(),
        GeneralName::DnsName(text) => format!("dns:{}", text.as_str()),
        GeneralName::Rfc822Name(text) => format!("email:{}", text.as_str()),
        GeneralName::UniformResourceIdentifier(text) => format!("uri:{}", text.as_str()),
        _ => "name of another form".to_string(),
    }
}

/// Whole seconds of `accuracy`, plus one when milliseconds or microseconds
/// are present; 0 when absent. Sub-second fields must be 1 to 999.
fn accuracy_seconds(accuracy: Option<&Accuracy>) -> Result<i64, Finding> {
    let Some(accuracy) = accuracy else {
        return Ok(0);
    };
    let subsecond = [accuracy.millis, accuracy.micros];
    if subsecond
        .iter()
        .flatten()
        .any(|value| !(1..=MAX_SUBSECOND).contains(value))
    {
        return Err(malformed("accuracy sub-second field out of range"));
    }
    let seconds = accuracy.seconds.unwrap_or(0);
    let seconds = i64::try_from(seconds).map_err(|_| malformed("accuracy out of range"))?;
    let round_up = i64::from(subsecond.iter().any(Option::is_some));
    seconds
        .checked_add(round_up)
        .ok_or_else(|| malformed("accuracy out of range"))
}
