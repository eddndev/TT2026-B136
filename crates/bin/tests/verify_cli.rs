//! End-to-end checks of `verify` through the compiled binary, over the
//! real authority, signer, and timestamp authority that the shared
//! fixture prepares. Covers the fully valid report, the JSON form, and
//! one failure per verification component.

mod verify_fixture;

use std::fs;
use std::process::Command;

use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};
use verify_fixture::{exe, path_with_suffix, pki_dir, run_checked, stdout_of, Fixture};

/// Validity window of the fixture certificate, read through
/// `pki show --json`, so instants relative to it stay meaningful no
/// matter when the certificate was issued or for how long.
fn certificate_window(fixture: &Fixture) -> (OffsetDateTime, OffsetDateTime) {
    let output = run_checked(
        Command::new(exe())
            .args(["pki", "--scripts-dir"])
            .arg(pki_dir())
            .arg("show")
            .arg(&fixture.certificate)
            .arg("--json"),
        "pki show",
    );
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("pki show --json prints one object");
    let instant = |key: &str| {
        OffsetDateTime::parse(body[key].as_str().unwrap(), &Rfc3339)
            .expect("pki show prints rfc 3339 instants")
    };
    (instant("not_before"), instant("not_after"))
}

fn rfc3339(instant: OffsetDateTime) -> String {
    instant.format(&Rfc3339).expect("the instant formats")
}

#[test]
fn an_intact_document_verifies_valid_on_every_component() {
    let fixture = Fixture::build();
    let crl = fixture.crl.to_str().unwrap().to_string();
    let output = fixture.verify(&fixture.document, true, &["--crl", &crl]);
    let text = stdout_of(&output);
    assert!(
        output.status.success(),
        "verify must exit zero on a valid verdict: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    for line in [
        "integrity:   passed",
        "signature:   passed",
        "certificate: passed",
        "timestamp:   passed",
        "verdict:     valid",
    ] {
        assert!(text.contains(line), "missing {line:?} in:\n{text}");
    }
    assert!(
        text.contains("revocation checked"),
        "the certificate line must state the revocation check: {text}"
    );
}

#[test]
fn the_json_report_carries_digest_components_and_verdict() {
    let fixture = Fixture::build();
    let output = fixture.verify(&fixture.document, true, &["--json"]);
    assert!(output.status.success());
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--json prints one object");

    let document_bytes = fs::read(&fixture.document).unwrap();
    let expected_digest = {
        use domain::crypto::DocumentHasher;
        infrastructure::RingSha256Hasher::new()
            .hash_bytes(&document_bytes)
            .to_hex()
    };
    assert_eq!(body["digest"], expected_digest.as_str());
    for component in ["integrity", "signature", "certificate", "timestamp"] {
        assert_eq!(
            body[component]["status"], "passed",
            "component {component} must pass: {body}"
        );
        assert!(body[component]["detail"].as_str().is_some());
    }
    assert_eq!(body["verdict"], "valid");
}

#[test]
fn a_flipped_document_byte_fails_the_signature_component_with_its_cause() {
    let fixture = Fixture::build();
    let tampered = fixture.dir.path().join("tampered.txt");
    let mut bytes = fs::read(&fixture.document).unwrap();
    bytes[7] ^= 0xff;
    fs::write(&tampered, bytes).unwrap();

    let output = fixture.verify(&tampered, true, &[]);
    assert!(
        !output.status.success(),
        "a not-valid verdict must exit nonzero"
    );
    let text = stdout_of(&output);
    assert!(
        text.contains("signature:   failed"),
        "the signature component must fail: {text}"
    );
    assert!(
        text.contains("signature does not match document and key"),
        "the cause must be reported: {text}"
    );
    assert!(text.contains("verdict:     not valid"), "report: {text}");
}

#[test]
fn an_evaluation_time_after_expiry_reports_the_expired_status() {
    let fixture = Fixture::build();
    let (_, not_after) = certificate_window(&fixture);
    let after_expiry = rfc3339(not_after + Duration::days(1));
    let output = fixture.verify(&fixture.document, true, &["--at", &after_expiry]);
    assert!(!output.status.success());
    let text = stdout_of(&output);
    assert!(
        text.contains("certificate: failed") && text.contains("expired"),
        "the expired status must be reported: {text}"
    );
    assert!(
        text.contains("does not by itself invalidate"),
        "the wording must leave the legal reading to the operator: {text}"
    );
    assert!(
        text.contains("signature:   passed"),
        "the signature itself still verifies: {text}"
    );
}

#[test]
fn an_evaluation_time_before_issuance_reports_not_yet_valid() {
    let fixture = Fixture::build();
    let (not_before, _) = certificate_window(&fixture);
    let before_issuance = rfc3339(not_before - Duration::days(1));
    let output = fixture.verify(&fixture.document, true, &["--at", &before_issuance]);
    assert!(!output.status.success());
    let text = stdout_of(&output);
    assert!(
        text.contains("certificate: failed") && text.contains("not yet valid"),
        "the not-yet-valid status must be reported: {text}"
    );
}

#[test]
fn a_revoked_certificate_reports_its_serial_from_the_fresh_crl() {
    let fixture = Fixture::build();
    run_checked(
        Command::new(exe())
            .args(["pki", "--scripts-dir"])
            .arg(pki_dir())
            .args(["revoke", "--serial", &fixture.serial])
            .env("PKI_CA_DIR", &fixture.ca_dir),
        "pki revoke",
    );
    run_checked(
        Command::new(exe())
            .args(["pki", "--scripts-dir"])
            .arg(pki_dir())
            .arg("gen-crl")
            .env("PKI_CA_DIR", &fixture.ca_dir),
        "pki gen-crl",
    );

    let crl = fixture.crl.to_str().unwrap().to_string();
    let output = fixture.verify(&fixture.document, true, &["--crl", &crl]);
    assert!(!output.status.success());
    let text = stdout_of(&output);
    assert!(
        text.contains("certificate: failed")
            && text.contains(&format!("revoked (serial {})", fixture.serial)),
        "the revoked status must name the serial: {text}"
    );
}

#[test]
fn without_a_token_the_dependent_components_are_skipped_with_reasons() {
    let fixture = Fixture::build();
    let output = fixture.verify(&fixture.document, false, &[]);
    assert!(
        output.status.success(),
        "skipped components must not poison the verdict: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = stdout_of(&output);
    assert!(
        text.contains("integrity:   skipped") && text.contains("no independent reference"),
        "integrity must say what it checked: {text}"
    );
    assert!(
        text.contains("timestamp:   skipped") && text.contains("no timestamp token supplied"),
        "the timestamp skip reason must be explicit: {text}"
    );
    assert!(text.contains("verdict:     valid"), "report: {text}");
}

#[test]
fn a_token_over_a_different_document_fails_the_integrity_component() {
    let fixture = Fixture::build();
    let other = fixture.dir.path().join("otro.txt");
    fs::write(&other, b"un documento distinto\n").unwrap();
    run_checked(
        Command::new(exe())
            .arg("timestamp")
            .arg(&other)
            .arg("--mock")
            .arg("--pki-dir")
            .arg(pki_dir())
            .env("TSA_DIR", &fixture.tsa_dir),
        "timestamp of the other document",
    );

    let other_token = path_with_suffix(&other, ".tsr");
    let output = Command::new(exe())
        .arg("verify")
        .arg(&fixture.document)
        .arg("--sig")
        .arg(&fixture.signature)
        .arg("--cert")
        .arg(&fixture.certificate)
        .arg("--ca")
        .arg(&fixture.root)
        .arg("--tsr")
        .arg(&other_token)
        .output()
        .expect("the binary runs");
    assert!(!output.status.success());
    let text = stdout_of(&output);
    assert!(
        text.contains("integrity:   failed") && text.contains("differs"),
        "the integrity mismatch must be reported: {text}"
    );
}

#[test]
fn a_certificate_from_another_authority_reports_an_untrusted_issuer() {
    let fixture = Fixture::build();
    // A second, unrelated root created directly with openssl.
    let other_root_key = fixture.dir.path().join("otra-raiz.key.pem");
    let other_root = fixture.dir.path().join("otra-raiz.crt.pem");
    run_checked(
        Command::new("openssl").args([
            "genpkey",
            "-quiet",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:3072",
            "-out",
            other_root_key.to_str().unwrap(),
        ]),
        "unrelated root key generation",
    );
    run_checked(
        Command::new("openssl").args([
            "req",
            "-x509",
            "-key",
            other_root_key.to_str().unwrap(),
            "-subj",
            "/CN=Otra Raiz",
            "-days",
            "1",
            "-sha256",
            "-out",
            other_root.to_str().unwrap(),
        ]),
        "unrelated root issuance",
    );

    let output = Command::new(exe())
        .arg("verify")
        .arg(&fixture.document)
        .arg("--sig")
        .arg(&fixture.signature)
        .arg("--cert")
        .arg(&fixture.certificate)
        .arg("--ca")
        .arg(&other_root)
        .arg("--tsr")
        .arg(&fixture.token)
        .arg("--tsa-cert")
        .arg(&fixture.root)
        .output()
        .expect("the binary runs");
    assert!(!output.status.success());
    let text = stdout_of(&output);
    assert!(
        text.contains("certificate: failed") && text.contains("untrusted issuer"),
        "the untrusted issuer must be reported: {text}"
    );
}
