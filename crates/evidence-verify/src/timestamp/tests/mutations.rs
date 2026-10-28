//! Single-byte and truncation mutations of a valid response. The verdicts
//! agree with `openssl ts -verify` on the same mutated bytes: every mutation
//! except a status of grantedWithMods is rejected there too.

use super::*;
use crate::der_walk;
use crate::timestamp::attributes::SignedAttributes;
use crate::timestamp::structure;

const SIGNING_TIME: &str = "1.2.840.113549.1.9.5";

/// Offset of `inner` inside `outer`; `inner` must borrow from `outer`.
fn offset(outer: &[u8], inner: &[u8]) -> usize {
    let at = inner.as_ptr() as usize - outer.as_ptr() as usize;
    assert!(at + inner.len() <= outer.len());
    at
}

fn flipped(input: &[u8], at: usize) -> Vec<u8> {
    let mut out = input.to_vec();
    out[at] ^= 0x01;
    out
}

fn verdict(input: &[u8]) -> Result<TokenFacts, Finding> {
    verify(input, &document_digest(), ADMITTED)
}

fn expect_failed(input: &[u8], code: &str) {
    let finding = expect_err(verdict(input));
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, code, "{finding:?}");
}

/// Offset of the status INTEGER value octet of a response.
fn status_offset(response: &[u8]) -> usize {
    let outer = der_walk::read_exact(response).unwrap();
    let (status_info, _) = der_walk::read(outer.value).unwrap();
    let (status, _) = der_walk::read(status_info.value).unwrap();
    assert_eq!(status.value.len(), 1);
    offset(response, status.value)
}

#[test]
fn a_flipped_byte_in_the_signed_attributes_breaks_the_signature() {
    let token = structure::token_of(V1).unwrap();
    let parts = structure::signed_token(token).unwrap();
    let attributes = SignedAttributes::parse(parts.signer.signed_attrs.value).unwrap();
    let oid = SIGNING_TIME.parse().unwrap();
    let time = attributes.single(oid).unwrap().unwrap();
    expect_failed(
        &flipped(V1, offset(V1, time.value)),
        "token.signature_invalid",
    );
}

#[test]
fn a_flipped_byte_in_the_encapsulated_content_breaks_the_message_digest() {
    let token = structure::token_of(V1).unwrap();
    let parts = structure::signed_token(token).unwrap();
    let last = offset(V1, parts.econtent) + parts.econtent.len() - 1;
    expect_failed(&flipped(V1, last), "token.message_digest_mismatch");
}

#[test]
fn a_flipped_byte_in_the_signature_is_rejected() {
    let token = structure::token_of(V2).unwrap();
    let parts = structure::signed_token(token).unwrap();
    let signature = parts.signer.signature;
    for at in [0, signature.len() / 2, signature.len() - 1] {
        let input = flipped(V2, offset(V2, signature) + at);
        expect_failed(&input, "token.signature_invalid");
    }
}

#[test]
fn a_truncated_token_is_malformed() {
    for length in [V1.len() - 1, V1.len() / 2, 10, 1] {
        expect_failed(&V1[..length], "token.malformed");
    }
    expect_failed(&BARE[..BARE.len() - 1], "token.malformed");
}

#[test]
fn a_status_other_than_granted_is_failed() {
    let at = status_offset(V1);
    for status in [2u8, 3, 4, 5] {
        let mut input = V1.to_vec();
        input[at] = status;
        expect_failed(&input, "token.not_granted");
    }
}

#[test]
fn a_status_of_granted_with_modifications_is_admitted() {
    let mut input = V2.to_vec();
    input[status_offset(V2)] = 1;
    assert!(verdict(&input).is_ok());
}

#[test]
fn a_granted_response_without_a_token_is_failed() {
    let outer = der_walk::read_exact(V1).unwrap();
    let (status_info, _) = der_walk::read(outer.value).unwrap();
    let mut input = vec![0x30, status_info.raw.len() as u8];
    input.extend_from_slice(status_info.raw);
    expect_failed(&input, "token.missing");
}

#[test]
fn a_flipped_byte_in_an_embedded_certificate_is_rejected() {
    let token = structure::token_of(V1).unwrap();
    let parts = structure::signed_token(token).unwrap();
    let signer = parts.certificates[0];
    let last = offset(V1, signer.raw) + signer.raw.len() - 1;
    expect_failed(&flipped(V1, last), "token.ess_mismatch");
}
