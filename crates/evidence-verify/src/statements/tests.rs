use super::*;
use crate::report::Outcome;

fn binding() -> BindingStatement {
    BindingStatement {
        deployment: [0x11; 16],
        root_sha256: [0x22; 32],
        trust_revision: 0x0102_0304,
        account: [0x33; 16],
        account_revision: 0x0506_0708_090a_0b0c,
        authentication_generation: 0x0506_0708_090a_0b00,
        binding: [0x44; 16],
        certificate_sha256: [0x55; 32],
    }
}

fn consent() -> ConsentStatement {
    let b = binding();
    ConsentStatement {
        deployment: b.deployment,
        root_sha256: b.root_sha256,
        trust_revision: b.trust_revision,
        account: b.account,
        account_revision: b.account_revision,
        authentication_generation: b.authentication_generation,
        binding: b.binding,
        certificate_sha256: b.certificate_sha256,
        operation: [0x66; 16],
        case: [0x77; 16],
        document: [0x88; 16],
        document_version: 0x0d0e_0f10,
        document_sha256: [0x99; 32],
        signature_sha256: [0xaa; 32],
    }
}

/// The prefix written field by field with literal offsets, independent of the
/// encoder, so the layout itself is under test.
fn hand_prefix(magic: &[u8; 8], bytes: &mut [u8]) {
    bytes[0..8].copy_from_slice(magic);
    bytes[8] = 1;
    bytes[9] = 1;
    bytes[10..26].fill(0x11);
    bytes[26..58].fill(0x22);
    bytes[58..62].copy_from_slice(&[1, 2, 3, 4]);
    bytes[62..78].fill(0x33);
    bytes[78..86].copy_from_slice(&[5, 6, 7, 8, 9, 10, 11, 12]);
    bytes[86..94].copy_from_slice(&[5, 6, 7, 8, 9, 10, 11, 0]);
    bytes[94..110].fill(0x44);
    bytes[110..142].fill(0x55);
}

fn hand_binding() -> [u8; 142] {
    let mut bytes = [0; 142];
    hand_prefix(b"DOCBIND1", &mut bytes);
    bytes
}

fn hand_consent() -> [u8; 258] {
    let mut bytes = [0; 258];
    hand_prefix(b"DOCSIGN1", &mut bytes);
    bytes[142..158].fill(0x66);
    bytes[158..174].fill(0x77);
    bytes[174..190].fill(0x88);
    bytes[190..194].copy_from_slice(&[13, 14, 15, 16]);
    bytes[194..226].fill(0x99);
    bytes[226..258].fill(0xaa);
    bytes
}

fn binding_code(bytes: &[u8]) -> &'static str {
    let finding = decode_binding(bytes).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed);
    finding.code
}

fn consent_code(bytes: &[u8]) -> &'static str {
    let finding = decode_consent(bytes).unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed);
    finding.code
}

fn with_prefix_counters(revision: u64, generation: u64) -> ([u8; 142], [u8; 258]) {
    let mut b = hand_binding();
    let mut c = hand_consent();
    for bytes in [&mut b[..], &mut c[..]] {
        bytes[78..86].copy_from_slice(&revision.to_be_bytes());
        bytes[86..94].copy_from_slice(&generation.to_be_bytes());
    }
    (b, c)
}

#[test]
fn lengths_match_the_domain_layout() {
    assert_eq!(BINDING_LEN, 142);
    assert_eq!(CONSENT_LEN, 258);
}

#[test]
fn encoders_write_the_domain_byte_offsets() {
    assert_eq!(encode_binding(&binding()), hand_binding());
    assert_eq!(encode_consent(&consent()), hand_consent());
}

#[test]
fn decoders_read_the_domain_byte_offsets() {
    assert_eq!(decode_binding(&hand_binding()).unwrap(), binding());
    assert_eq!(decode_consent(&hand_consent()).unwrap(), consent());
}

#[test]
fn encode_then_decode_round_trips() {
    let b = binding();
    assert_eq!(decode_binding(&encode_binding(&b)).unwrap(), b);
    let c = consent();
    assert_eq!(decode_consent(&encode_consent(&c)).unwrap(), c);
}

#[test]
fn wrong_lengths_are_rejected_before_any_field() {
    let b = hand_binding();
    let c = hand_consent();
    assert_eq!(binding_code(&b[..141]), "statement.length");
    assert_eq!(
        binding_code(&[b.as_slice(), &[0]].concat()),
        "statement.length"
    );
    assert_eq!(binding_code(&[]), "statement.length");
    assert_eq!(consent_code(&c[..257]), "statement.length");
    assert_eq!(
        consent_code(&[c.as_slice(), &[0]].concat()),
        "statement.length"
    );
    assert_eq!(consent_code(&[]), "statement.length");
    assert_eq!(binding_code(&c), "statement.length");
    assert_eq!(consent_code(&b), "statement.length");
}

#[test]
fn a_swapped_purpose_of_the_right_length_is_rejected() {
    let mut b = hand_binding();
    b[..8].copy_from_slice(b"DOCSIGN1");
    assert_eq!(binding_code(&b), "statement.purpose");
    let mut c = hand_consent();
    c[..8].copy_from_slice(b"DOCBIND1");
    assert_eq!(consent_code(&c), "statement.purpose");
}

#[test]
fn changed_magic_purpose_or_format_bytes_are_rejected() {
    for (position, code) in [
        (0, "statement.purpose"),
        (7, "statement.purpose"),
        (8, "statement.format"),
        (9, "statement.format"),
    ] {
        let mut b = hand_binding();
        b[position] ^= 1;
        assert_eq!(binding_code(&b), code, "binding byte {position}");
        let mut c = hand_consent();
        c[position] ^= 1;
        assert_eq!(consent_code(&c), code, "consent byte {position}");
    }
}

#[test]
fn nil_prefix_identities_are_rejected() {
    for range in [10..26, 62..78, 94..110] {
        let mut b = hand_binding();
        b[range.clone()].fill(0);
        assert_eq!(binding_code(&b), "statement.nil_identity");
        let mut c = hand_consent();
        c[range].fill(0);
        assert_eq!(consent_code(&c), "statement.nil_identity");
    }
}

#[test]
fn nil_consent_references_are_rejected() {
    for range in [142..158, 158..174, 174..190] {
        let mut c = hand_consent();
        c[range].fill(0);
        assert_eq!(consent_code(&c), "statement.nil_identity");
    }
}

#[test]
fn zero_trust_revision_is_rejected() {
    let mut b = hand_binding();
    b[58..62].fill(0);
    assert_eq!(binding_code(&b), "statement.trust_revision");
    let mut c = hand_consent();
    c[58..62].fill(0);
    assert_eq!(consent_code(&c), "statement.trust_revision");
}

#[test]
fn account_revision_above_the_signed_range_is_rejected() {
    let (b, c) = with_prefix_counters(i64::MAX as u64 + 1, 0);
    assert_eq!(binding_code(&b), "statement.account_version");
    assert_eq!(consent_code(&c), "statement.account_version");
    let (b, c) = with_prefix_counters(u64::MAX, u64::MAX);
    assert_eq!(binding_code(&b), "statement.account_version");
    assert_eq!(consent_code(&c), "statement.account_version");
}

#[test]
fn generation_above_revision_is_rejected() {
    let (b, c) = with_prefix_counters(5, 6);
    assert_eq!(binding_code(&b), "statement.account_version");
    assert_eq!(consent_code(&c), "statement.account_version");
    let mut b = hand_binding();
    b[86..94].fill(255);
    assert_eq!(binding_code(&b), "statement.account_version");
}

#[test]
fn counter_boundaries_are_admitted() {
    let (b, c) = with_prefix_counters(i64::MAX as u64, i64::MAX as u64);
    let decoded = decode_binding(&b).unwrap();
    assert_eq!(decoded.account_revision, i64::MAX as u64);
    assert_eq!(decoded.authentication_generation, i64::MAX as u64);
    assert_eq!(
        decode_consent(&c).unwrap().account_revision,
        i64::MAX as u64
    );
    let (b, c) = with_prefix_counters(0, 0);
    assert_eq!(decode_binding(&b).unwrap().account_revision, 0);
    assert_eq!(decode_consent(&c).unwrap().authentication_generation, 0);
    let mut b = hand_binding();
    b[58..62].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(decode_binding(&b).unwrap().trust_revision, u32::MAX);
}

#[test]
fn zero_document_version_is_rejected() {
    let mut c = hand_consent();
    c[190..194].fill(0);
    assert_eq!(consent_code(&c), "statement.document_version");
    c[193] = 1;
    assert_eq!(decode_consent(&c).unwrap().document_version, 1);
}

#[test]
fn rejection_details_name_fields_without_echoing_values() {
    let mut c = hand_consent();
    c[158..174].fill(0);
    let finding = decode_consent(&c).unwrap_err();
    assert_eq!(finding.detail, "case");
    let mut b = hand_binding();
    b[94..110].fill(0);
    assert_eq!(decode_binding(&b).unwrap_err().detail, "binding");
}

#[test]
fn digests_are_opaque_and_may_take_any_value() {
    let mut c = hand_consent();
    for range in [26..58, 110..142, 194..226, 226..258] {
        c[range].fill(0);
    }
    let decoded = decode_consent(&c).unwrap();
    assert_eq!(decoded.root_sha256, [0; 32]);
    assert_eq!(decoded.signature_sha256, [0; 32]);
}

#[test]
fn uuid_text_is_lowercase_and_hyphenated() {
    assert_eq!(uuid_text([0; 16]), "00000000-0000-0000-0000-000000000000");
    let uuid = [
        0xa0, 0xb1, 0xc2, 0xd3, 0xe4, 0xf5, 0x06, 0x17, 0x28, 0x39, 0x4a, 0x5b, 0x6c, 0x7d, 0x8e,
        0xff,
    ];
    assert_eq!(uuid_text(uuid), "a0b1c2d3-e4f5-0617-2839-4a5b6c7d8eff");
}

/// The statements signed by the fixture holder, decoded and linked to the
/// fixture files they name. Expected digests are computed from those files,
/// never copied, because the fixture generator produces new bytes each run.
#[test]
fn fixture_statements_decode_and_reference_their_artifacts() {
    use sha2::{Digest, Sha256};
    let sha256 = |bytes: &[u8]| -> [u8; 32] { Sha256::digest(bytes).into() };
    let binding = decode_binding(include_bytes!(
        "../../tests/fixtures/certificate-binding.bin"
    ))
    .expect("fixture binding statement");
    let consent = decode_consent(include_bytes!("../../tests/fixtures/document-consent.bin"))
        .expect("fixture consent statement");

    let root = sha256(include_bytes!("../../tests/fixtures/root.der"));
    let holder = sha256(include_bytes!("../../tests/fixtures/holder.der"));
    assert_eq!(binding.root_sha256, root);
    assert_eq!(binding.certificate_sha256, holder);
    assert_eq!(
        consent.document_sha256,
        sha256(include_bytes!("../../tests/fixtures/document.bin"))
    );
    assert_eq!(
        consent.signature_sha256,
        sha256(include_bytes!("../../tests/fixtures/holder.sig"))
    );
    let prefix = |statement: &ConsentStatement| BindingStatement {
        deployment: statement.deployment,
        root_sha256: statement.root_sha256,
        trust_revision: statement.trust_revision,
        account: statement.account,
        account_revision: statement.account_revision,
        authentication_generation: statement.authentication_generation,
        binding: statement.binding,
        certificate_sha256: statement.certificate_sha256,
    };
    assert_eq!(prefix(&consent), binding);
}
