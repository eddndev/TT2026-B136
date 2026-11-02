use super::*;
use crate::report::Outcome;
use crate::statements::{decode_binding, decode_consent};
use sha2::{Digest, Sha256};

/// The shared vector, also read by the domain codec tests.
const VECTOR: &str = include_str!("../../../domain/tests/fixtures/contract_statement_vectors.json");

/// The flat JSON object recorded for `name`; vector objects hold no nesting.
fn object(name: &str) -> &'static str {
    let start = VECTOR.find(&format!("\"{name}\": {{")).unwrap();
    let body = &VECTOR[start..];
    &body[..body.find('}').unwrap()]
}

/// The scalar text of `key` inside `object`, without quotes.
fn scalar<'a>(object: &'a str, key: &str) -> &'a str {
    let marker = format!("\"{key}\": ");
    let rest = &object[object.find(&marker).unwrap() + marker.len()..];
    if let Some(text) = rest.strip_prefix('"') {
        return &text[..text.find('"').unwrap()];
    }
    let end = rest.find([',', '\n']).unwrap_or(rest.len());
    rest[..end].trim()
}

fn unhex(text: &str) -> Vec<u8> {
    let text = text.replace('-', "");
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

fn fixed<const N: usize>(object: &str, key: &str) -> [u8; N] {
    unhex(scalar(object, key)).try_into().unwrap()
}

fn number<T: core::str::FromStr>(object: &str, key: &str) -> T
where
    T::Err: core::fmt::Debug,
{
    scalar(object, key).parse().unwrap()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn encoded(name: &str) -> Vec<u8> {
    unhex(scalar(object(name), "hex"))
}

fn expected_request() -> ContractRequest {
    let inputs = object("request_inputs");
    ContractRequest {
        deployment: fixed(inputs, "deployment_id"),
        request: fixed(inputs, "request_id"),
        case: fixed(inputs, "case_id"),
        document: fixed(inputs, "document_id"),
        document_version: number(inputs, "document_version"),
        document_sha256: fixed(inputs, "document_sha256"),
        client_account: fixed(inputs, "client_account_id"),
        firm_account: fixed(inputs, "firm_account_id"),
        requester_account: fixed(inputs, "requester_account_id"),
        client_certificate_sha256: fixed(inputs, "client_certificate_sha256"),
        firm_certificate_sha256: fixed(inputs, "firm_certificate_sha256"),
        created_at: number(inputs, "created_at_seconds"),
        expires_at: number(inputs, "expires_at_seconds"),
    }
}

fn expected_statement(party: &str) -> PartyStatement {
    let values = object(party);
    let inputs = object("request_inputs");
    PartyStatement {
        deployment: fixed(inputs, "deployment_id"),
        root_sha256: fixed(values, "root_sha256"),
        trust_revision: number(values, "trust_revision"),
        account: fixed(values, "account_id"),
        account_revision: number(values, "account_revision"),
        authentication_generation: number(values, "authentication_generation"),
        binding: fixed(values, "binding_id"),
        certificate_sha256: fixed(values, "certificate_sha256"),
        operation: fixed(values, "operation_id"),
        request: fixed(inputs, "request_id"),
        request_sha256: sha256(&encoded("request")),
        document_sha256: fixed(inputs, "document_sha256"),
        position: number(values, "position"),
        prior_signature_sha256: fixed(values, "prior_signature_sha256"),
        prior_token_sha256: fixed(values, "prior_token_sha256"),
    }
}

fn request_detail(bytes: &[u8]) -> String {
    let finding = decode_request(bytes).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "contract.request_schema");
    finding.detail
}

fn statement_code(bytes: &[u8]) -> &'static str {
    let finding = decode_party_statement(bytes).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed);
    finding.code
}

fn changed(bytes: &[u8], range: core::ops::Range<usize>, value: u8) -> Vec<u8> {
    let mut bytes = bytes.to_vec();
    bytes[range].fill(value);
    bytes
}

#[test]
fn lengths_match_the_frozen_layout() {
    assert_eq!(REQUEST_LEN, 240);
    assert_eq!(PARTY_STATEMENT_LEN, 303);
}

#[test]
fn the_shared_request_vector_decodes_and_re_encodes() {
    let bytes = encoded("request");
    assert_eq!(bytes.len(), number::<usize>(object("request"), "length"));
    assert_eq!(sha256(&bytes), fixed(object("request"), "sha256"));
    let request = decode_request(&bytes).unwrap();
    assert_eq!(request, expected_request());
    assert_eq!(encode_request(&request).as_slice(), bytes.as_slice());
}

#[test]
fn both_shared_party_vectors_decode_and_re_encode() {
    for (name, party) in [
        ("client_statement", "client_party"),
        ("firm_statement", "firm_party"),
    ] {
        let bytes = encoded(name);
        assert_eq!(bytes.len(), number::<usize>(object(name), "length"));
        assert_eq!(sha256(&bytes), fixed(object(name), "sha256"), "{name}");
        let statement = decode_party_statement(&bytes).unwrap();
        assert_eq!(statement, expected_statement(party), "{name}");
        assert_eq!(
            encode_party_statement(&statement).as_slice(),
            bytes.as_slice()
        );
    }
    let firm = object("firm_party");
    let signature = unhex(scalar(firm, "prior_signature_hex"));
    let token = unhex(scalar(firm, "prior_token_hex"));
    let statement = decode_party_statement(&encoded("firm_statement")).unwrap();
    assert_eq!(statement.prior_signature_sha256, sha256(&signature));
    assert_eq!(statement.prior_token_sha256, sha256(&token));
    let client = decode_party_statement(&encoded("client_statement")).unwrap();
    assert_eq!(client.prior_signature_sha256, [0; 32]);
    assert_eq!(client.prior_token_sha256, [0; 32]);
}

#[test]
fn wrong_lengths_magic_and_format_are_rejected_first() {
    let request = encoded("request");
    let statement = encoded("firm_statement");
    assert_eq!(request_detail(&request[..239]), "length");
    assert_eq!(
        request_detail(&[request.as_slice(), &[0]].concat()),
        "length"
    );
    assert_eq!(request_detail(&statement), "length");
    assert_eq!(statement_code(&statement[..302]), "statement.length");
    assert_eq!(statement_code(&request), "statement.length");
    assert_eq!(statement_code(&[]), "statement.length");
    for (position, detail, code) in [
        (0, "purpose", "statement.purpose"),
        (7, "purpose", "statement.purpose"),
        (8, "format", "statement.format"),
        (9, "format", "statement.format"),
    ] {
        assert_eq!(
            request_detail(&changed(&request, position..position + 1, 0x7f)),
            detail
        );
        assert_eq!(
            statement_code(&changed(&statement, position..position + 1, 0x7f)),
            code
        );
    }
}

#[test]
fn every_other_signed_statement_is_rejected_both_ways() {
    let mut others = vec![
        magic_only(b"DOCBIND1", 142),
        magic_only(b"OWNCERT1", 150),
        magic_only(b"OWNAUTH1", 182),
        magic_only(b"DOCSIGN1", 258),
    ];
    let mut swapped = encoded("firm_statement");
    swapped[..8].copy_from_slice(b"CTRTREQ1");
    others.push(swapped);
    let mut swapped = encoded("request");
    swapped[..8].copy_from_slice(b"CTRTSIG1");
    others.push(swapped);
    for bytes in &others {
        assert!(decode_request(bytes).is_err());
        assert!(decode_party_statement(bytes).is_err());
    }
    for name in ["request", "client_statement", "firm_statement"] {
        assert!(decode_binding(&encoded(name)).is_err());
        assert!(decode_consent(&encoded(name)).is_err());
    }
}

#[test]
fn request_fields_reapply_identity_party_and_validity_rules() {
    let bytes = encoded("request");
    for (range, name) in [
        (10..26, "deployment"),
        (26..42, "request"),
        (42..58, "case"),
        (58..74, "document"),
        (110..126, "client_account"),
        (126..142, "firm_account"),
        (142..158, "requester_account"),
    ] {
        assert_eq!(request_detail(&changed(&bytes, range, 0)), name);
    }
    assert_eq!(
        request_detail(&changed(&bytes, 74..78, 0)),
        "document_version"
    );
    for (offset, value, detail) in [
        (238, 1, "parties"),
        (238, 3, "parties"),
        (239, 0, "completion_rule"),
        (239, 2, "completion_rule"),
    ] {
        assert_eq!(
            request_detail(&changed(&bytes, offset..offset + 1, value)),
            detail
        );
    }
    for (from, to, detail) in [
        (110..126, 126, "same_party"),
        (110..126, 142, "same_party"),
        (158..190, 190, "same_certificate"),
        (230..238, 222, "validity"),
    ] {
        let mut invalid = bytes.clone();
        invalid.copy_within(from, to);
        assert_eq!(request_detail(&invalid), detail);
    }
    let mut owner_requests = bytes.clone();
    owner_requests.copy_within(126..142, 142);
    assert!(decode_request(&owner_requests).is_ok());
}

#[test]
fn party_statement_fields_reapply_prefix_and_position_rules() {
    let firm = encoded("firm_statement");
    let client = encoded("client_statement");
    for range in [10..26, 62..78, 94..110, 142..158, 158..174] {
        assert_eq!(
            statement_code(&changed(&firm, range, 0)),
            "statement.nil_identity"
        );
    }
    assert_eq!(
        statement_code(&changed(&firm, 58..62, 0)),
        "statement.trust_revision"
    );
    assert_eq!(
        statement_code(&changed(&firm, 78..86, 0xff)),
        "statement.account_version"
    );
    for value in [0, 3, 0xff] {
        assert_eq!(
            statement_code(&changed(&firm, 238..239, value)),
            "statement.position"
        );
    }
    for range in [239..271, 271..303] {
        assert_eq!(
            statement_code(&changed(&firm, range, 0)),
            "statement.position"
        );
    }
    assert_eq!(
        statement_code(&changed(&client, 238..239, 2)),
        "statement.position"
    );
    assert_eq!(
        statement_code(&changed(&firm, 238..239, 1)),
        "statement.not_canonical"
    );
    assert_eq!(
        statement_code(&changed(&client, 302..303, 1)),
        "statement.not_canonical"
    );
}

fn magic_only(magic: &[u8; 8], length: usize) -> Vec<u8> {
    let mut bytes = vec![0x5a; length];
    bytes[..8].copy_from_slice(magic);
    bytes[8] = 1;
    bytes[9] = 1;
    bytes
}
