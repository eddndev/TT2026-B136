//! The signed attributes of the SignerInfo, read without re-encoding, and
//! the checks of the attributes a token must carry.

use core::cmp::Ordering;

use der::asn1::{GeneralizedTime, ObjectIdentifier, UtcTime};
use der::{Decode, Encode};

use super::structure::{der, expect_tag, OCTET_STRING, OID, SEQUENCE, SET};
use super::{
    ess, malformed, ID_CONTENT_TYPE, ID_CT_TST_INFO, ID_MESSAGE_DIGEST, ID_SIGNING_CERTIFICATE,
    ID_SIGNING_CERTIFICATE_V2,
};
use crate::certificate::{self, Certificate};
use crate::der_walk::{self, Tlv};
use crate::report::Finding;

/// `id-signingTime`, the signed attribute with the signer's claimed time.
const ID_SIGNING_TIME: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.5");
/// Identifier octets of the two `Time` choices.
const UTC_TIME: u8 = 0x17;
const GENERALIZED_TIME: u8 = 0x18;

/// Most signed attributes admitted.
const MAX_ATTRIBUTES: usize = 16;
/// Most values admitted in one attribute.
const MAX_VALUES: usize = 8;

/// The attributes of `signedAttrs` with their values as encoded.
#[derive(Debug)]
pub(super) struct SignedAttributes<'a> {
    list: Vec<(ObjectIdentifier, Vec<Tlv<'a>>)>,
}

impl<'a> SignedAttributes<'a> {
    /// Read the content octets of `signedAttrs`.
    ///
    /// The set must be in DER order, which also rules out two identical
    /// attributes; each attribute type may appear once and must have at
    /// least one value.
    pub(super) fn parse(value: &'a [u8]) -> Result<Self, Finding> {
        let elements = der_walk::children(value, MAX_ATTRIBUTES).map_err(der)?;
        if elements.is_empty() {
            return Err(malformed("the signed attributes are empty"));
        }
        if elements
            .windows(2)
            .any(|pair| der_set_order(pair[0].raw, pair[1].raw) != Ordering::Less)
        {
            return Err(malformed("the signed attributes are not in DER order"));
        }
        let mut list: Vec<(ObjectIdentifier, Vec<Tlv<'a>>)> = Vec::new();
        for element in elements {
            expect_tag(&element, SEQUENCE, "attribute")?;
            let parts = der_walk::children(element.value, 2).map_err(der)?;
            let [kind, values] = parts.as_slice() else {
                return Err(malformed("an attribute is not a type and a value set"));
            };
            expect_tag(kind, OID, "attribute type")?;
            expect_tag(values, SET, "attribute values")?;
            let kind = oid(*kind)?;
            let values = der_walk::children(values.value, MAX_VALUES).map_err(der)?;
            if values.is_empty() {
                return Err(malformed(format!("attribute {kind} has no value")));
            }
            if list.iter().any(|(seen, _)| *seen == kind) {
                return Err(malformed(format!("attribute {kind} appears twice")));
            }
            list.push((kind, values));
        }
        Ok(Self { list })
    }

    /// The single value of the attribute `kind`, or `None` when absent. An
    /// attribute with more than one value is malformed.
    pub(super) fn single(&self, kind: ObjectIdentifier) -> Result<Option<Tlv<'a>>, Finding> {
        match self.list.iter().find(|(seen, _)| *seen == kind) {
            None => Ok(None),
            Some((_, values)) if values.len() == 1 => Ok(Some(values[0])),
            Some(_) => Err(malformed(format!("attribute {kind} has several values"))),
        }
    }
}

/// Decode an OBJECT IDENTIFIER element.
pub(super) fn oid(tlv: Tlv<'_>) -> Result<ObjectIdentifier, Finding> {
    ObjectIdentifier::from_der(tlv.raw).map_err(|_| malformed("invalid OBJECT IDENTIFIER"))
}

/// Order of two encodings in a DER SET OF: compared as octet strings, the
/// shorter padded at its end with zero octets (X.690, 11.6).
fn der_set_order(left: &[u8], right: &[u8]) -> Ordering {
    let length = left.len().max(right.len());
    (0..length)
        .map(|at| {
            let a = left.get(at).copied().unwrap_or(0);
            let b = right.get(at).copied().unwrap_or(0);
            a.cmp(&b)
        })
        .find(|ordering| ordering.is_ne())
        .unwrap_or(Ordering::Equal)
}

/// `contentType` must be present and equal to `id-ct-TSTInfo`.
pub(super) fn check_content_type(attributes: &SignedAttributes<'_>) -> Result<(), Finding> {
    let mismatch = |detail: &str| Finding::failed("token.content_type_mismatch", detail);
    let value = attributes
        .single(ID_CONTENT_TYPE)?
        .ok_or_else(|| mismatch("the content type attribute is missing"))?;
    expect_tag(&value, OID, "the content type attribute")?;
    let oid = oid(value)?;
    if oid != ID_CT_TST_INFO {
        return Err(mismatch(&format!("content type {oid}")));
    }
    Ok(())
}

/// `messageDigest` must be present and equal to SHA-256 of `econtent`.
pub(super) fn check_message_digest(
    attributes: &SignedAttributes<'_>,
    econtent: &[u8],
) -> Result<(), Finding> {
    let mismatch = |detail: &str| Finding::failed("token.message_digest_mismatch", detail);
    let value = attributes
        .single(ID_MESSAGE_DIGEST)?
        .ok_or_else(|| mismatch("the message digest attribute is missing"))?;
    expect_tag(&value, OCTET_STRING, "the message digest attribute")?;
    if value.value != certificate::sha256(econtent) {
        return Err(mismatch(
            "the message digest is not the SHA-256 of the encapsulated content",
        ));
    }
    Ok(())
}

/// At least one ESS signing certificate attribute must be present and each
/// present one must name `signer`. Returns which of `SigningCertificate`
/// and `SigningCertificateV2` were present.
pub(super) fn check_ess(
    attributes: &SignedAttributes<'_>,
    signer: &Certificate,
) -> Result<(bool, bool), Finding> {
    let v1 = attributes.single(ID_SIGNING_CERTIFICATE)?;
    let v2 = attributes.single(ID_SIGNING_CERTIFICATE_V2)?;
    if v1.is_none() && v2.is_none() {
        return Err(Finding::failed(
            "token.ess_missing",
            "no ESS signing certificate attribute",
        ));
    }
    if let Some(value) = v1 {
        ess::check_v1(value.raw, signer)?;
    }
    if let Some(value) = v2 {
        ess::check_v2(value.raw, signer)?;
    }
    Ok((v1.is_some(), v2.is_some()))
}

/// The `signingTime` attribute in Unix seconds, or `None` when absent. It
/// is reported and never relied on, but as a signed attribute it must be a
/// strict DER UTCTime or GeneralizedTime.
pub(super) fn signing_time(attributes: &SignedAttributes<'_>) -> Result<Option<i64>, Finding> {
    let Some(value) = attributes.single(ID_SIGNING_TIME)? else {
        return Ok(None);
    };
    let invalid = || malformed("the signing time attribute is not a strict time");
    let (duration, encoded) = match value.tag {
        UTC_TIME => {
            let time = UtcTime::from_der(value.raw).map_err(|_| invalid())?;
            (time.to_unix_duration(), time.to_der())
        }
        GENERALIZED_TIME => {
            let time = GeneralizedTime::from_der(value.raw).map_err(|_| invalid())?;
            (time.to_unix_duration(), time.to_der())
        }
        _ => return Err(invalid()),
    };
    if encoded.ok().as_deref() != Some(value.raw) {
        return Err(invalid());
    }
    i64::try_from(duration.as_secs())
        .map(Some)
        .map_err(|_| invalid())
}
