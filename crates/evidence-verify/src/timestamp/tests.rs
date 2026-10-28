use super::*;
use crate::report::Outcome;

mod mutations;
mod parts;
mod shape;
mod signed_attributes;
mod tst_info;

const V1: &[u8] = include_bytes!("../../tests/fixtures/token-ess-v1.tsr");
const V2: &[u8] = include_bytes!("../../tests/fixtures/token-ess-v2.tsr");
const BARE: &[u8] = include_bytes!("../../tests/fixtures/token-bare-v2.der");
const OTHER_POLICY: &[u8] = include_bytes!("../../tests/fixtures/token-other-policy.tsr");
const FOREIGN: &[u8] = include_bytes!("../../tests/fixtures/token-foreign.tsr");
const OTHER_DIGEST: &[u8] = include_bytes!("../../tests/fixtures/token-other-digest.tsr");
const TSA: &[u8] = include_bytes!("../../tests/fixtures/tsa.der");
const ROOT: &[u8] = include_bytes!("../../tests/fixtures/root.der");
const FOREIGN_TSA: &[u8] = include_bytes!("../../tests/fixtures/foreign-tsa.der");
const FOREIGN_ROOT: &[u8] = include_bytes!("../../tests/fixtures/foreign-root.der");
const DOCUMENT: &[u8] = include_bytes!("../../tests/fixtures/document.bin");
const FACTS: &str = include_str!("../../tests/fixtures/fixtures.json");

/// The policy of the internal authority, as configured in `pki/tsa.cnf`.
const INTERNAL_POLICY: &str = "1.3.6.1.4.1.13762.3";
const ADMITTED: &[&str] = &[INTERNAL_POLICY];

/// Every token fixture with its file name, as listed in `fixtures.json`.
const ALL: [(&str, &[u8]); 6] = [
    ("token-ess-v1.tsr", V1),
    ("token-ess-v2.tsr", V2),
    ("token-bare-v2.der", BARE),
    ("token-other-policy.tsr", OTHER_POLICY),
    ("token-foreign.tsr", FOREIGN),
    ("token-other-digest.tsr", OTHER_DIGEST),
];

/// Verdicts of `openssl ts -verify -digest <digest> -in <file> -CAfile
/// <anchor>` (with `-token_in` for the bare token) on the fixtures, where
/// `document` is the digest of `document.bin` and `own` the digest each
/// token records in `fixtures.json`. OpenSSL applies neither the admitted
/// policy nor pinning, so these rows are compared with [`verify`] admitting
/// the token's own policy and with the anchor recorded for the token.
const OPENSSL_VERDICTS: [(&str, &str, bool); 8] = [
    ("token-ess-v1.tsr", "document", true),
    ("token-ess-v2.tsr", "document", true),
    ("token-bare-v2.der", "document", true),
    ("token-other-policy.tsr", "document", true),
    ("token-foreign.tsr", "document", true),
    ("token-other-digest.tsr", "document", false),
    ("token-other-digest.tsr", "own", true),
    ("token-ess-v1.tsr", "other", false),
];

/// Read one scalar recorded for the token `file` in the `tokens` object of
/// `fixtures.json`. Token objects are flat, so each ends at its first
/// closing brace. Strings are returned without quotes.
fn token_fact(file: &str, key: &str) -> Option<String> {
    let tokens = &FACTS[FACTS.find("\"tokens\": {")?..];
    let start = tokens.find(&format!("\"{file}\": {{"))?;
    let object = &tokens[start..];
    let object = &object[..object.find('}')?];
    let marker = format!("\"{key}\": ");
    let rest = &object[object.find(&marker)? + marker.len()..];
    if let Some(text) = rest.strip_prefix('"') {
        return Some(text[..text.find('"')?].to_string());
    }
    let end = rest.find([',', '\n']).unwrap_or(rest.len());
    Some(rest[..end].trim().to_string())
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0, "odd hexadecimal length");
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hexadecimal digit"))
        .collect()
}

fn digest_of(file: &str) -> [u8; 32] {
    let text = token_fact(file, "covered_sha256").expect("covered digest recorded");
    hex_bytes(&text).try_into().expect("32-byte digest")
}

fn document_digest() -> [u8; 32] {
    crate::certificate::sha256(DOCUMENT)
}

fn hex_upper(bytes: &[u8]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
    match hex.trim_start_matches('0') {
        "" => "0".to_string(),
        trimmed => trimmed.to_string(),
    }
}

fn certificate_file(name: &str) -> &'static [u8] {
    match name {
        "tsa.der" => TSA,
        "root.der" => ROOT,
        "foreign-tsa.der" => FOREIGN_TSA,
        "foreign-root.der" => FOREIGN_ROOT,
        other => panic!("unexpected certificate fixture {other}"),
    }
}

fn tsa() -> Certificate {
    crate::certificate::parse(TSA).unwrap()
}

fn root() -> Certificate {
    crate::certificate::parse(ROOT).unwrap()
}

fn assert_finding<T: core::fmt::Debug>(result: Result<T, Finding>, outcome: Outcome, code: &str) {
    let finding = result.expect_err("expected a finding");
    assert_eq!(
        (finding.outcome, finding.code),
        (outcome, code),
        "{finding:?}"
    );
}

fn expect_err(result: Result<TokenFacts, Finding>) -> Finding {
    match result {
        Ok(facts) => panic!("expected a finding, got facts for policy {}", facts.policy),
        Err(finding) => finding,
    }
}

#[test]
fn every_document_token_records_the_document_digest() {
    for (file, _) in ALL {
        if token_fact(file, "covers").as_deref() == Some("document.bin") {
            assert_eq!(digest_of(file), document_digest(), "{file}");
        }
    }
}

#[test]
fn fixtures_verify_with_the_facts_recorded_for_them() {
    for (file, bytes) in ALL {
        let policy = token_fact(file, "policy").expect("policy recorded");
        let facts = verify(bytes, &digest_of(file), &[policy.as_str()])
            .unwrap_or_else(|finding| panic!("{file}: {finding:?}"));
        let gen_time: i64 = token_fact(file, "gen_time").unwrap().parse().unwrap();
        let accuracy: i64 = token_fact(file, "accuracy_seconds")
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(facts.gen_time, gen_time, "{file}");
        assert_eq!(facts.accuracy_seconds, accuracy, "{file}");
        assert_eq!(facts.policy, policy, "{file}");
        assert_eq!(
            hex_upper(&facts.serial),
            token_fact(file, "serial_hex").unwrap(),
            "{file}"
        );
        assert_eq!(
            facts.nonce.is_some(),
            token_fact(file, "nonce_present").as_deref() == Some("true"),
            "{file}"
        );
        let signer = certificate_file(&token_fact(file, "signer").unwrap());
        let anchor = certificate_file(&token_fact(file, "anchor").unwrap());
        assert_eq!(facts.signer_certificate_der, signer, "{file}");
        assert_eq!(
            facts.certificates,
            vec![signer.to_vec(), anchor.to_vec()],
            "{file}"
        );
        let ess = token_fact(file, "ess").unwrap();
        assert_eq!(facts.ess_v1, ess == "signing-certificate", "{file}");
        assert_eq!(facts.ess_v2, ess == "signing-certificate-v2", "{file}");
    }
}

#[test]
fn verdicts_match_openssl_on_the_same_fixtures() {
    for (file, digest, expected) in OPENSSL_VERDICTS {
        let bytes = ALL
            .iter()
            .find(|(name, _)| *name == file)
            .map(|(_, bytes)| *bytes)
            .unwrap();
        let covered = match digest {
            "document" => document_digest(),
            "own" => digest_of(file),
            _ => crate::certificate::sha256(b"another object"),
        };
        let policy = token_fact(file, "policy").unwrap();
        let result = verify(bytes, &covered, &[policy.as_str()]);
        assert_eq!(result.is_ok(), expected, "{file} over {digest}: {result:?}");
    }
}

#[test]
fn response_and_bare_token_yield_the_signer_of_the_internal_authority() {
    for bytes in [V1, V2, BARE] {
        let facts = verify(bytes, &document_digest(), ADMITTED).unwrap();
        assert_eq!(facts.signer_certificate_der, TSA);
        assert_eq!(facts.policy, INTERNAL_POLICY);
    }
}

#[test]
fn ess_kind_is_reported_per_token() {
    let v1 = verify(V1, &document_digest(), ADMITTED).unwrap();
    assert!(v1.ess_v1 && !v1.ess_v2);
    let v2 = verify(V2, &document_digest(), ADMITTED).unwrap();
    assert!(!v2.ess_v1 && v2.ess_v2);
    let bare = verify(BARE, &document_digest(), ADMITTED).unwrap();
    assert!(!bare.ess_v1 && bare.ess_v2);
}

#[test]
fn a_policy_outside_the_admitted_list_is_not_supported() {
    let finding = expect_err(verify(OTHER_POLICY, &document_digest(), ADMITTED));
    assert_eq!(finding.outcome, Outcome::NotSupported);
    assert_eq!(finding.code, "token.policy_not_admitted");
    let finding = expect_err(verify(V1, &document_digest(), &[]));
    assert_eq!(finding.code, "token.policy_not_admitted");
    let both = [INTERNAL_POLICY, "1.3.6.1.4.1.13762.4"];
    assert!(verify(OTHER_POLICY, &document_digest(), &both).is_ok());
}

#[test]
fn a_foreign_token_verifies_structurally_and_names_its_own_signer() {
    let facts = verify(FOREIGN, &document_digest(), ADMITTED).unwrap();
    assert_eq!(facts.signer_certificate_der, FOREIGN_TSA);
    assert_eq!(facts.certificates[1], FOREIGN_ROOT);
}

#[test]
fn a_token_over_another_digest_is_an_imprint_mismatch() {
    let finding = expect_err(verify(OTHER_DIGEST, &document_digest(), ADMITTED));
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "token.imprint_mismatch");
    let other = crate::certificate::sha256(b"another object");
    let finding = expect_err(verify(V2, &other, ADMITTED));
    assert_eq!(finding.code, "token.imprint_mismatch");
}

#[test]
fn a_policy_outside_the_admitted_list_is_not_supported_before_other_checks() {
    let other = crate::certificate::sha256(b"another object");
    let finding = expect_err(verify(OTHER_POLICY, &other, ADMITTED));
    assert_eq!(finding.outcome, Outcome::NotSupported);
    assert_eq!(finding.code, "token.policy_not_admitted");
    let token = parse(OTHER_POLICY).unwrap();
    let signature = token.parts.signer.signature;
    let at = signature.as_ptr() as usize - OTHER_POLICY.as_ptr() as usize;
    let mut broken = OTHER_POLICY.to_vec();
    broken[at] ^= 0x01;
    let finding = expect_err(verify(&broken, &document_digest(), ADMITTED));
    assert_eq!(finding.code, "token.policy_not_admitted");
}

#[test]
fn parse_locates_the_signer_before_any_cryptographic_check() {
    let token = parse(FOREIGN).unwrap();
    let signature = token.parts.signer.signature;
    let at = signature.as_ptr() as usize - FOREIGN.as_ptr() as usize;
    let mut broken = FOREIGN.to_vec();
    broken[at] ^= 0x01;
    let token = parse(&broken).unwrap();
    assert_eq!(token.signer_certificate().der, FOREIGN_TSA);
    let embedded: Vec<&[u8]> = token
        .certificates()
        .iter()
        .map(|cert| cert.der.as_slice())
        .collect();
    assert_eq!(embedded, [FOREIGN_TSA, FOREIGN_ROOT]);
    let finding = expect_err(token.verify(&document_digest(), ADMITTED));
    assert_eq!(finding.outcome, Outcome::Failed);
    assert_eq!(finding.code, "token.signature_invalid");
    let facts = parse(FOREIGN)
        .unwrap()
        .verify(&document_digest(), ADMITTED)
        .unwrap();
    assert_eq!(
        facts,
        verify(FOREIGN, &document_digest(), ADMITTED).unwrap()
    );
}

#[test]
fn facts_report_signing_time_ordering_and_authority_name() {
    for (file, bytes) in ALL {
        let policy = token_fact(file, "policy").unwrap();
        let facts = verify(bytes, &digest_of(file), &[policy.as_str()]).unwrap();
        let ordering = token_fact(file, "ordering").expect("ordering recorded");
        assert_eq!(facts.ordering.to_string(), ordering, "{file}");
        assert_eq!(facts.tsa, None, "{file}");
        let signing_time = facts.signing_time.expect("signingTime present");
        let window =
            facts.gen_time - facts.accuracy_seconds..=facts.gen_time + facts.accuracy_seconds;
        assert!(window.contains(&signing_time), "{file}: {signing_time}");
    }
}

#[test]
fn input_that_is_not_a_token_is_malformed() {
    for input in [&b""[..], b"\x30\x00", b"\x04\x01\x00", b"not der at all"] {
        let finding = expect_err(verify(input, &document_digest(), ADMITTED));
        assert_eq!(finding.outcome, Outcome::Failed, "{input:?}");
        assert_eq!(finding.code, "token.malformed", "{input:?}");
    }
    let mut trailing = V1.to_vec();
    trailing.push(0);
    let finding = expect_err(verify(&trailing, &document_digest(), ADMITTED));
    assert_eq!(finding.code, "token.malformed");
}

#[test]
fn an_oversized_input_is_not_supported() {
    let input = vec![0u8; TOKEN_LIMIT + 1];
    let finding = expect_err(verify(&input, &document_digest(), ADMITTED));
    assert_eq!(finding.outcome, Outcome::NotSupported);
    assert_eq!(finding.code, "token.too_large");
}
