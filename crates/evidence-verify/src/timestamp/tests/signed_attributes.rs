//! Checks of the signed attributes over constructed encodings.

use super::*;
use crate::timestamp::attributes::{self, SignedAttributes};

/// One attribute `SEQUENCE { OID, SET { values } }` with the given OID
/// content octets.
fn attribute(oid: &[u8], values: &[&[u8]]) -> Vec<u8> {
    let mut set: Vec<u8> = values.concat();
    set.splice(0..0, [0x31, set.len() as u8]);
    let mut out = vec![0x06, oid.len() as u8];
    out.extend_from_slice(oid);
    out.extend_from_slice(&set);
    out.splice(0..0, [0x30, out.len() as u8]);
    out
}

const NULL: &[u8] = &[0x05, 0x00];

#[test]
fn signed_attributes_must_be_sorted_unique_and_single_valued() {
    let low = attribute(&[0x2A, 0x03, 0x01], &[NULL]);
    let high = attribute(&[0x2A, 0x03, 0x02], &[NULL, NULL]);
    let sorted = [low.clone(), high.clone()].concat();
    let attributes = SignedAttributes::parse(&sorted).unwrap();
    assert!(attributes
        .single("1.2.3.1".parse().unwrap())
        .unwrap()
        .is_some());
    assert!(attributes
        .single("1.2.3.3".parse().unwrap())
        .unwrap()
        .is_none());
    let finding = attributes.single("1.2.3.2".parse().unwrap()).unwrap_err();
    assert_eq!(finding.code, "token.malformed");
    let unsorted = [high.clone(), low.clone()].concat();
    assert_eq!(
        SignedAttributes::parse(&unsorted).unwrap_err().code,
        "token.malformed"
    );
    let duplicate = [
        low.clone(),
        attribute(&[0x2A, 0x03, 0x01], &[&[0x05, 0x00]]),
    ]
    .concat();
    assert!(SignedAttributes::parse(&duplicate).is_err());
    let same_type = [low, attribute(&[0x2A, 0x03, 0x01], &[&[0x02, 0x01, 0x00]])].concat();
    assert!(SignedAttributes::parse(&same_type).is_err());
    let no_values = attribute(&[0x2A, 0x03, 0x01], &[]);
    assert!(SignedAttributes::parse(&no_values).is_err());
}
/// OID content octets of the attributes the checks read.
const CONTENT_TYPE: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x03];
const MESSAGE_DIGEST: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x04];
const SIGNING_TIME: &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x05];
/// Encoded OBJECT IDENTIFIER values of `id-ct-TSTInfo` and `id-data`.
const TST_INFO_OID: &[u8] = &[
    0x06, 0x0B, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x09, 0x10, 0x01, 0x04,
];
const DATA_OID: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01,
];

fn octets(value: &[u8]) -> Vec<u8> {
    super::shape::encode(0x04, value)
}

fn with_attribute<T>(
    oid: &[u8],
    value: &[u8],
    check: impl Fn(&SignedAttributes<'_>) -> Result<T, Finding>,
) -> Result<T, Finding> {
    let encoded = attribute(oid, &[value]);
    check(&SignedAttributes::parse(&encoded).unwrap())
}

#[test]
fn the_content_type_attribute_must_name_tst_info() {
    let check = |attributes: &SignedAttributes<'_>| attributes::check_content_type(attributes);
    assert!(with_attribute(CONTENT_TYPE, TST_INFO_OID, check).is_ok());
    let result = with_attribute(CONTENT_TYPE, DATA_OID, check);
    assert_finding(result, Outcome::Failed, "token.content_type_mismatch");
    let result = with_attribute(CONTENT_TYPE, NULL, check);
    assert_finding(result, Outcome::Failed, "token.malformed");
    let result = with_attribute(MESSAGE_DIGEST, TST_INFO_OID, check);
    assert_finding(result, Outcome::Failed, "token.content_type_mismatch");
}

#[test]
fn the_message_digest_attribute_must_be_the_sha256_of_the_content() {
    let digest = octets(&crate::certificate::sha256(b"content"));
    let check = |attributes: &SignedAttributes<'_>| {
        attributes::check_message_digest(attributes, b"content")
    };
    assert!(with_attribute(MESSAGE_DIGEST, &digest, check).is_ok());
    let other = octets(&crate::certificate::sha256(b"other"));
    let result = with_attribute(MESSAGE_DIGEST, &other, check);
    assert_finding(result, Outcome::Failed, "token.message_digest_mismatch");
    let result = with_attribute(MESSAGE_DIGEST, NULL, check);
    assert_finding(result, Outcome::Failed, "token.malformed");
    let result = with_attribute(CONTENT_TYPE, &digest, check);
    assert_finding(result, Outcome::Failed, "token.message_digest_mismatch");
}

#[test]
fn an_ess_signing_certificate_attribute_is_required() {
    let check = |attributes: &SignedAttributes<'_>| attributes::check_ess(attributes, &tsa());
    let result = with_attribute(CONTENT_TYPE, TST_INFO_OID, check);
    assert_finding(result, Outcome::Failed, "token.ess_missing");
}

#[test]
fn the_signing_time_is_reported_when_present_and_well_formed() {
    let check = |attributes: &SignedAttributes<'_>| attributes::signing_time(attributes);
    let utc = super::shape::encode(0x17, b"261028053152Z");
    assert_eq!(
        with_attribute(SIGNING_TIME, &utc, check),
        Ok(Some(1_793_165_512))
    );
    let generalized = super::shape::encode(0x18, b"20261028053152Z");
    assert_eq!(
        with_attribute(SIGNING_TIME, &generalized, check),
        Ok(Some(1_793_165_512))
    );
    assert_eq!(with_attribute(CONTENT_TYPE, &utc, check), Ok(None));
    for invalid in [
        super::shape::encode(0x17, b"2610280531Z"),
        super::shape::encode(0x17, b"261028053152"),
        NULL.to_vec(),
    ] {
        let result = with_attribute(SIGNING_TIME, &invalid, check);
        assert_finding(result, Outcome::Failed, "token.malformed");
    }
}
