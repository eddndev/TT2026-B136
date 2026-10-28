//! Encoding strictness below the outer certificate structure.
//!
//! The outer round trip of [`crate::certificate::parse`] cannot see inside
//! extension values, which are OCTET STRING contents, so the profile decodes
//! each value it reads and admits it only in its single DER form.

use der::asn1::OctetString;
use der::oid::AssociatedOid;
use der::{Decode, Encode};
use x509_cert::ext::pkix::{BasicConstraints, KeyUsage};
use x509_cert::Certificate as X509;

use super::{parsed, HOLDER, TSA};
use crate::certificate::{check_profile, parse, Role};
use crate::report::Outcome;

/// `der` with the value of its extension of type `T` replaced by `value`.
fn with_raw_value<T: AssociatedOid>(der: &[u8], value: &[u8]) -> Vec<u8> {
    let mut certificate = X509::from_der(der).unwrap();
    let extension = certificate
        .tbs_certificate
        .extensions
        .as_mut()
        .unwrap()
        .iter_mut()
        .find(|extension| extension.extn_id == T::OID)
        .unwrap();
    extension.extn_value = OctetString::new(value).unwrap();
    certificate.to_der().unwrap()
}

fn assert_profile_fails(der: &[u8], role: Role, code: &str) {
    let finding = check_profile(&parsed(der), role).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, code, "{finding:?}");
}

#[test]
fn basic_constraints_with_an_encoded_default_are_not_der() {
    // cA FALSE is the DEFAULT, so DER omits it: SEQUENCE { BOOLEAN FALSE }.
    let changed = with_raw_value::<BasicConstraints>(HOLDER, &[0x30, 0x03, 0x01, 0x01, 0x00]);
    assert_profile_fails(
        &changed,
        Role::IndividualSigner,
        "cert.profile.basic_constraints",
    );
}

#[test]
fn key_usage_with_trailing_zero_bits_is_not_der() {
    // digitalSignature and nonRepudiation: DER is 03 02 06 C0; the same
    // bits padded with a zero octet are another encoding of the value.
    assert_eq!(
        KeyUsage::from_der(&[0x03, 0x02, 0x06, 0xC0]).unwrap(),
        parsed_usage(HOLDER)
    );
    let padded = with_raw_value::<KeyUsage>(HOLDER, &[0x03, 0x03, 0x00, 0xC0, 0x00]);
    assert_profile_fails(&padded, Role::IndividualSigner, "cert.profile.key_usage");
    let unpruned = with_raw_value::<KeyUsage>(HOLDER, &[0x03, 0x02, 0x00, 0xC0]);
    assert_profile_fails(&unpruned, Role::IndividualSigner, "cert.profile.key_usage");
}

#[test]
fn key_usage_bits_past_the_named_bits_are_not_dropped() {
    // digitalSignature plus bit 9, which has no name; a decoder that
    // truncates unknown bits would read digitalSignature alone.
    let extended = with_raw_value::<KeyUsage>(TSA, &[0x03, 0x03, 0x06, 0x80, 0x40]);
    assert_profile_fails(&extended, Role::TimeStamping, "cert.profile.key_usage");
}

fn parsed_usage(der: &[u8]) -> KeyUsage {
    let certificate = X509::from_der(der).unwrap();
    let extension = certificate
        .tbs_certificate
        .extensions
        .unwrap()
        .into_iter()
        .find(|extension| extension.extn_id == KeyUsage::OID)
        .unwrap();
    KeyUsage::from_der(extension.extn_value.as_bytes()).unwrap()
}

#[test]
fn nesting_beyond_the_bound_is_a_failed_malformed_certificate() {
    let mut nested: Vec<u8> = Vec::new();
    for _ in 0..crate::der_walk::MAX_DEPTH + 1 {
        let mut outer = vec![0x30, u8::try_from(nested.len()).unwrap()];
        outer.extend_from_slice(&nested);
        nested = outer;
    }
    let finding = parse(&nested).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, "cert.malformed", "{finding:?}");
    assert_eq!(finding.detail, "der.too_deep", "{finding:?}");
}
