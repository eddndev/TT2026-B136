//! Structural mutations of a valid response, rebuilt with consistent
//! lengths so that each one reaches the rule it targets instead of failing
//! as truncated input.

use super::*;
use crate::der_walk;
use crate::timestamp::attributes::SignedAttributes;
use crate::timestamp::structure;

/// Child indices of the SignedData inside a response: the token, its
/// `[0]` content and the SignedData itself.
const SIGNED_DATA: [usize; 3] = [1, 1, 0];
const CERTIFICATES: usize = 3;
const SIGNER_INFOS: usize = 4;
const NULL: &[u8] = &[0x05, 0x00];
/// `id-data`, a content type that is neither SignedData nor TSTInfo.
const ID_DATA: &[u8] = &[
    0x06, 0x09, 0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x07, 0x01,
];
/// `AlgorithmIdentifier { id-sha384, NULL }`.
const SHA384: &[u8] = &[
    0x30, 0x0D, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02, 0x05, 0x00,
];

pub(super) fn encode(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    if value.len() < 0x80 {
        out.push(value.len() as u8);
    } else {
        let length = (value.len() as u32).to_be_bytes();
        let skip = length.iter().take_while(|octet| **octet == 0).count();
        out.push(0x80 | (4 - skip) as u8);
        out.extend_from_slice(&length[skip..]);
    }
    out.extend_from_slice(value);
    out
}

/// Replace the element that `path` reaches (child indices from the outer
/// element of `input`) with `change` of its encoding, re-encoding every
/// enclosing length. An empty replacement removes the element.
fn edit(input: &[u8], path: &[usize], change: &dyn Fn(&[u8]) -> Vec<u8>) -> Vec<u8> {
    let outer = der_walk::read_exact(input).unwrap();
    let Some((&at, rest)) = path.split_first() else {
        return change(outer.raw);
    };
    let children = der_walk::children(outer.value, usize::MAX).unwrap();
    assert!(at < children.len(), "path leaves the structure");
    let mut value = Vec::new();
    for (index, child) in children.iter().enumerate() {
        if index == at {
            value.extend(edit(child.raw, rest, change));
        } else {
            value.extend_from_slice(child.raw);
        }
    }
    encode(outer.tag, &value)
}

fn path(tail: &[usize]) -> Vec<usize> {
    [&SIGNED_DATA[..], tail].concat()
}

fn signer_path(tail: &[usize]) -> Vec<usize> {
    path(&[[SIGNER_INFOS, 0].as_slice(), tail].concat())
}

fn outcome_of(input: &[u8]) -> (Outcome, &'static str) {
    let finding = expect_err(verify(input, &document_digest(), ADMITTED));
    (finding.outcome, finding.code)
}

const MALFORMED_TOKEN: (Outcome, &str) = (Outcome::Failed, "token.malformed");

#[test]
fn the_rebuilder_preserves_an_unchanged_response() {
    assert_eq!(edit(V1, &signer_path(&[0]), &|raw| raw.to_vec()), V1);
}

#[test]
fn signed_data_must_be_version_3() {
    let input = edit(V1, &path(&[0]), &|_| vec![0x02, 0x01, 0x04]);
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn the_token_must_be_signed_data_over_tst_info() {
    let input = edit(V1, &[1, 0], &|_| ID_DATA.to_vec());
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
    let input = edit(V1, &path(&[2, 0]), &|_| ID_DATA.to_vec());
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
    let input = edit(V1, &path(&[2, 1]), &|_| Vec::new());
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn a_response_with_an_extra_element_is_malformed() {
    let input = edit(V1, &[1], &|raw| [raw, NULL].concat());
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
    let input = edit(V1, &path(&[SIGNER_INFOS]), &|raw| [raw, NULL].concat());
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn a_token_without_certificates_names_no_signer() {
    let input = edit(V1, &path(&[CERTIFICATES]), &|_| Vec::new());
    assert_eq!(
        outcome_of(&input),
        (Outcome::Failed, "token.signer_not_found")
    );
}

#[test]
fn too_many_embedded_certificates_are_malformed() {
    let input = edit(V1, &path(&[CERTIFICATES, 0]), &|raw| raw.repeat(16));
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn a_certificate_choice_other_than_a_certificate_is_not_supported() {
    let input = edit(V1, &path(&[CERTIFICATES, 1]), &|raw| {
        let mut out = raw.to_vec();
        out[0] = 0xA1;
        out
    });
    assert_eq!(
        outcome_of(&input),
        (Outcome::NotSupported, "token.certificate_choice")
    );
}

#[test]
fn ignored_revocation_data_and_unsigned_attributes_do_not_change_the_verdict() {
    let input = edit(V1, &path(&[CERTIFICATES]), &|raw| {
        [raw, &[0xA1, 0x00]].concat()
    });
    assert!(verify(&input, &document_digest(), ADMITTED).is_ok());
    let unsigned = encode(
        0xA1,
        &encode(0x30, &[ID_DATA, &encode(0x31, NULL)].concat()),
    );
    let input = edit(V1, &signer_path(&[5]), &|raw| [raw, &unsigned].concat());
    assert!(verify(&input, &document_digest(), ADMITTED).is_ok());
}

#[test]
fn a_token_with_two_signers_is_failed() {
    let input = edit(V1, &signer_path(&[]), &|raw| [raw, raw].concat());
    assert_eq!(outcome_of(&input), (Outcome::Failed, "token.signer_count"));
    let input = edit(V1, &path(&[SIGNER_INFOS]), &|_| vec![0x31, 0x00]);
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn the_signed_attributes_must_carry_their_implicit_tag() {
    let input = edit(V1, &signer_path(&[3]), &|raw| {
        let mut out = raw.to_vec();
        out[0] = 0xA1;
        out
    });
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn the_signer_digest_algorithm_must_be_sha256_and_listed() {
    let input = edit(V1, &signer_path(&[2]), &|_| SHA384.to_vec());
    assert_eq!(
        outcome_of(&input),
        (Outcome::NotSupported, "token.digest_algorithm")
    );
    let input = edit(V1, &path(&[1, 0]), &|_| SHA384.to_vec());
    assert_eq!(outcome_of(&input), MALFORMED_TOKEN);
}

#[test]
fn a_flipped_byte_in_the_ess_attribute_breaks_the_signature() {
    for (bytes, oid) in [
        (V1, "1.2.840.113549.1.9.16.2.12"),
        (V2, "1.2.840.113549.1.9.16.2.47"),
    ] {
        let token = structure::token_of(bytes).unwrap();
        let parts = structure::signed_token(token).unwrap();
        let attributes = SignedAttributes::parse(parts.signer.signed_attrs.value).unwrap();
        let ess = attributes.single(oid.parse().unwrap()).unwrap().unwrap();
        let at = ess.raw.as_ptr() as usize - bytes.as_ptr() as usize + ess.raw.len() - 1;
        let mut input = bytes.to_vec();
        input[at] ^= 0x01;
        assert_eq!(
            outcome_of(&input),
            (Outcome::Failed, "token.signature_invalid")
        );
    }
}
