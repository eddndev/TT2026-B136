use super::*;
use crate::report::Outcome;

mod encoding;
mod mutations;

pub(super) const ROOT: &[u8] = include_bytes!("../../tests/fixtures/root.der");
pub(super) const FOREIGN_ROOT: &[u8] = include_bytes!("../../tests/fixtures/foreign-root.der");
pub(super) const SERVER_SIGNER: &[u8] = include_bytes!("../../tests/fixtures/server-signer.der");
pub(super) const HOLDER: &[u8] = include_bytes!("../../tests/fixtures/holder.der");
const REVOKED_HOLDER: &[u8] = include_bytes!("../../tests/fixtures/revoked-holder.der");
pub(super) const TSA: &[u8] = include_bytes!("../../tests/fixtures/tsa.der");
const FOREIGN_TSA: &[u8] = include_bytes!("../../tests/fixtures/foreign-tsa.der");
const TSA_NONCRITICAL_EKU: &[u8] = include_bytes!("../../tests/fixtures/tsa-noncritical-eku.der");
const SIGNER_CA_TRUE: &[u8] = include_bytes!("../../tests/fixtures/signer-ca-true.der");
const SIGNER_RSA2048: &[u8] = include_bytes!("../../tests/fixtures/signer-rsa2048.der");
const HOLDER_WITH_EKU: &[u8] = include_bytes!("../../tests/fixtures/holder-with-eku.der");
const ROOT_PEM: &[u8] = include_bytes!("../../tests/fixtures/root.pem");
const DOCUMENT: &[u8] = include_bytes!("../../tests/fixtures/document.bin");
const SERVER_SIGNER_SIG: &[u8] = include_bytes!("../../tests/fixtures/server-signer.sig");
const HOLDER_SIG: &[u8] = include_bytes!("../../tests/fixtures/holder.sig");
const FACTS: &str = include_str!("../../tests/fixtures/fixtures.json");

/// Every certificate fixture with its file name, as listed in `fixtures.json`.
const ALL: [(&str, &[u8]); 11] = [
    ("root.der", ROOT),
    ("foreign-root.der", FOREIGN_ROOT),
    ("server-signer.der", SERVER_SIGNER),
    ("holder.der", HOLDER),
    ("revoked-holder.der", REVOKED_HOLDER),
    ("tsa.der", TSA),
    ("foreign-tsa.der", FOREIGN_TSA),
    ("tsa-noncritical-eku.der", TSA_NONCRITICAL_EKU),
    ("signer-ca-true.der", SIGNER_CA_TRUE),
    ("signer-rsa2048.der", SIGNER_RSA2048),
    ("holder-with-eku.der", HOLDER_WITH_EKU),
];

/// Read one scalar recorded for `file` in `fixtures.json`.
///
/// The certificate objects of that file are flat, so the object of `file`
/// ends at its first closing brace. Strings are returned without quotes.
pub(super) fn fact(file: &str, key: &str) -> Option<String> {
    let start = FACTS.find(&format!("\"{file}\": {{"))?;
    let object = &FACTS[start..];
    let object = &object[..object.find('}')?];
    let marker = format!("\"{key}\": ");
    let rest = &object[object.find(&marker)? + marker.len()..];
    if let Some(text) = rest.strip_prefix('"') {
        return Some(text[..text.find('"')?].to_string());
    }
    let end = rest.find([',', '\n']).unwrap_or(rest.len());
    match rest[..end].trim() {
        "null" => None,
        value => Some(value.to_string()),
    }
}

fn fact_i64(file: &str, key: &str) -> i64 {
    fact(file, key).unwrap().parse().unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn parsed(der: &[u8]) -> Certificate {
    parse(der).unwrap()
}

fn assert_failed(result: Result<(), Finding>, code: &str) {
    let finding = result.unwrap_err();
    assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
    assert_eq!(finding.code, code, "{finding:?}");
}

#[test]
fn every_fixture_parses_with_the_recorded_facts() {
    for (file, der) in ALL {
        let certificate = parsed(der);
        assert_eq!(certificate.der, der, "{file}");
        assert_eq!(
            certificate.serial_hex(),
            fact(file, "serial_hex").unwrap(),
            "{file}"
        );
        assert_eq!(
            certificate.not_before,
            fact_i64(file, "not_before"),
            "{file}"
        );
        assert_eq!(certificate.not_after, fact_i64(file, "not_after"), "{file}");
        assert_eq!(
            certificate.subject.to_string(),
            fact(file, "subject").unwrap()
        );
        assert_eq!(
            certificate.issuer.to_string(),
            fact(file, "issuer").unwrap()
        );
        assert_eq!(
            certificate.subject_key_id.as_deref().map(hex),
            fact(file, "subject_key_id"),
            "{file}"
        );
        assert_eq!(
            certificate.authority_key_id.as_deref().map(hex),
            fact(file, "authority_key_id"),
            "{file}"
        );
        let bits = certificate.public_key.as_ref().map(|key| key.size() * 8);
        assert_eq!(bits, Some(fact_i64(file, "key_bits") as usize), "{file}");
        assert_eq!(hex(&sha256(der)), fact(file, "sha256").unwrap(), "{file}");
    }
}

#[test]
fn the_signed_part_is_the_exact_first_element_of_the_certificate() {
    let certificate = parsed(HOLDER);
    let outer = crate::der_walk::read_exact(HOLDER).unwrap();
    let first = crate::der_walk::read(outer.value).unwrap().0;
    assert_eq!(certificate.tbs_der, first.raw);
    assert_eq!(certificate.signature.len(), 384);
}

#[test]
fn malformed_input_is_a_failed_malformed_certificate() {
    let mut trailing = ROOT.to_vec();
    trailing.push(0);
    let mut non_der_boolean = ROOT.to_vec();
    let at = non_der_boolean
        .windows(3)
        .position(|window| window == [0x01, 0x01, 0xFF])
        .unwrap();
    non_der_boolean[at + 2] = 0x01;
    let inputs: [&[u8]; 6] = [
        b"",
        b"definitely not der",
        &ROOT[..ROOT.len() - 1],
        &trailing,
        ROOT_PEM,
        &non_der_boolean,
    ];
    for input in inputs {
        let finding = parse(input).unwrap_err();
        assert_eq!(finding.outcome, Outcome::Failed, "{finding:?}");
        assert_eq!(finding.code, "cert.malformed", "{finding:?}");
    }
}

#[test]
fn an_oversized_certificate_is_not_supported() {
    let finding = parse(&vec![0x30; CERTIFICATE_LIMIT + 1]).unwrap_err();
    assert_eq!(finding.outcome, Outcome::NotSupported);
    assert_eq!(finding.code, "cert.too_large");
}

#[test]
fn every_good_fixture_passes_its_role() {
    let cases = [
        (ROOT, Role::Root),
        (FOREIGN_ROOT, Role::Root),
        (SERVER_SIGNER, Role::ServerSealSigner),
        (HOLDER, Role::IndividualSigner),
        (REVOKED_HOLDER, Role::IndividualSigner),
        (TSA, Role::TimeStamping),
        (FOREIGN_TSA, Role::TimeStamping),
    ];
    for (der, role) in cases {
        assert_eq!(check_profile(&parsed(der), role), Ok(()), "{role:?}");
    }
}

#[test]
fn every_defective_fixture_fails_with_its_rule() {
    let cases = [
        (
            TSA_NONCRITICAL_EKU,
            Role::TimeStamping,
            "cert.profile.extended_key_usage",
        ),
        (
            SIGNER_CA_TRUE,
            Role::ServerSealSigner,
            "cert.profile.basic_constraints",
        ),
        (
            SIGNER_RSA2048,
            Role::ServerSealSigner,
            "cert.profile.key_size",
        ),
        (
            HOLDER_WITH_EKU,
            Role::IndividualSigner,
            "cert.profile.extensions",
        ),
    ];
    for (der, role, code) in cases {
        assert_failed(check_profile(&parsed(der), role), code);
    }
}

#[test]
fn a_certificate_fails_every_role_but_its_own() {
    let cases = [
        (ROOT, Role::Root),
        (SERVER_SIGNER, Role::ServerSealSigner),
        (HOLDER, Role::IndividualSigner),
        (TSA, Role::TimeStamping),
    ];
    let roles = [
        Role::Root,
        Role::ServerSealSigner,
        Role::IndividualSigner,
        Role::TimeStamping,
    ];
    for (der, own) in cases {
        let certificate = parsed(der);
        for role in roles.into_iter().filter(|role| *role != own) {
            let finding = check_profile(&certificate, role).unwrap_err();
            assert_eq!(finding.outcome, Outcome::Failed, "{own:?} as {role:?}");
            assert!(finding.code.starts_with("cert.profile."), "{finding:?}");
        }
    }
}

#[test]
fn issued_certificates_verify_under_their_issuer() {
    let root = parsed(ROOT);
    for der in [
        SERVER_SIGNER,
        HOLDER,
        REVOKED_HOLDER,
        TSA,
        TSA_NONCRITICAL_EKU,
    ] {
        assert_eq!(check_issued_by(&parsed(der), &root), Ok(()));
    }
    let foreign = parsed(FOREIGN_ROOT);
    assert_eq!(check_issued_by(&parsed(FOREIGN_TSA), &foreign), Ok(()));
}

#[test]
fn a_foreign_issuer_is_rejected() {
    let foreign = parsed(FOREIGN_ROOT);
    assert_failed(
        check_issued_by(&parsed(HOLDER), &foreign),
        "cert.issuer.name_mismatch",
    );
    assert_failed(
        check_issued_by(&parsed(FOREIGN_TSA), &parsed(ROOT)),
        "cert.issuer.name_mismatch",
    );
}

#[test]
fn an_issuer_with_another_key_identifier_is_rejected() {
    let root = parsed(ROOT);
    assert_failed(check_issued_by(&root, &root), "cert.issuer.key_id_mismatch");
    let mut other = parsed(ROOT);
    other.subject_key_id = parsed(FOREIGN_ROOT).subject_key_id;
    assert_failed(
        check_issued_by(&parsed(HOLDER), &other),
        "cert.issuer.key_id_mismatch",
    );
}

#[test]
fn an_altered_signature_or_signed_part_is_rejected() {
    let root = parsed(ROOT);
    let mut signature = SERVER_SIGNER.to_vec();
    *signature.last_mut().unwrap() ^= 1;
    let altered = parsed(&signature);
    assert_eq!(check_profile(&altered, Role::ServerSealSigner), Ok(()));
    assert_failed(
        check_issued_by(&altered, &root),
        "cert.issuer.signature_invalid",
    );
    let mut subject = SERVER_SIGNER.to_vec();
    let at = subject
        .windows(18)
        .position(|window| window == b"Server Seal Signer")
        .unwrap();
    subject[at] = b'T';
    let altered = parsed(&subject);
    assert_eq!(check_profile(&altered, Role::ServerSealSigner), Ok(()));
    assert_failed(
        check_issued_by(&altered, &root),
        "cert.issuer.signature_invalid",
    );
    let mut wrong_key = parsed(ROOT);
    wrong_key.public_key = parsed(FOREIGN_ROOT).public_key;
    assert_failed(
        check_issued_by(&parsed(HOLDER), &wrong_key),
        "cert.issuer.signature_invalid",
    );
}

#[test]
fn validity_covers_only_intervals_inside_the_window() {
    let holder = parsed(HOLDER);
    let (start, end) = (holder.not_before, holder.not_after);
    assert!(valid_over(&holder, start, end));
    assert!(valid_over(&holder, start + 10, start + 10));
    assert!(!valid_over(&holder, start - 1, start + 10));
    assert!(!valid_over(&holder, end - 10, end + 1));
    assert!(!valid_over(&holder, start + 20, start + 10));
}

#[test]
fn rsa_sha256_signatures_verify_only_with_the_right_key_and_message() {
    let signer = parsed(SERVER_SIGNER).public_key.unwrap();
    let holder = parsed(HOLDER).public_key.unwrap();
    assert!(verify_rsa_sha256(&signer, DOCUMENT, SERVER_SIGNER_SIG));
    assert!(verify_rsa_sha256(&holder, DOCUMENT, HOLDER_SIG));
    assert!(!verify_rsa_sha256(&holder, DOCUMENT, SERVER_SIGNER_SIG));
    assert!(!verify_rsa_sha256(
        &signer,
        b"another document",
        SERVER_SIGNER_SIG
    ));
    assert!(!verify_rsa_sha256(
        &signer,
        DOCUMENT,
        &SERVER_SIGNER_SIG[1..]
    ));
    let mut padded = vec![0];
    padded.extend_from_slice(SERVER_SIGNER_SIG);
    assert!(!verify_rsa_sha256(&signer, DOCUMENT, &padded));
}

#[test]
fn sha256_matches_a_known_vector() {
    assert_eq!(
        hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
