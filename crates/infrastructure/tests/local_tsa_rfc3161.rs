//! End-to-end tests of the local OpenSSL timestamp authority adapter
//! and the RFC 3161 verifier, driving the real scripts under `pki/`
//! against temporary CA and TSA directories and cross-checking the
//! issued tokens with the openssl command line tool.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use domain::crypto::timestamp::{TimestampService, TimestampVerification, TimestampVerifier};
use domain::crypto::{DocumentHasher, Sha256Digest};
use domain::DomainError;
use infrastructure::timestamp::token_info;
use infrastructure::{LocalOpensslTsa, Rfc3161Verifier, RingSha256Hasher};

fn pki_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

/// An initialized CA plus an issued TSA working directory, both inside
/// one temporary directory that is removed when the fixture drops.
struct TsaFixture {
    dir: tempfile::TempDir,
    tsa_dir: PathBuf,
}

impl TsaFixture {
    fn build() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ca_dir = dir.path().join("ca");
        let tsa_dir = dir.path().join("tsa");
        run_script("init-ca.sh", &ca_dir, &tsa_dir);
        run_script("issue-tsa-cert.sh", &ca_dir, &tsa_dir);
        Self { dir, tsa_dir }
    }

    fn adapter(&self) -> LocalOpensslTsa {
        LocalOpensslTsa::new(pki_dir().join("tsa.cnf"), &self.tsa_dir)
    }

    fn root_certificate_path(&self) -> PathBuf {
        self.dir.path().join("ca").join("ca.crt.pem")
    }

    fn scratch_path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }
}

fn run_script(name: &str, ca_dir: &Path, tsa_dir: &Path) {
    let output = Command::new("bash")
        .arg(pki_dir().join(name))
        .env("PKI_CA_DIR", ca_dir)
        .env("TSA_DIR", tsa_dir)
        .output()
        .expect("bash and the pki scripts are runnable");
    assert!(
        output.status.success(),
        "{name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sample_digest() -> Sha256Digest {
    RingSha256Hasher::new().hash_bytes(b"expediente de prueba")
}

#[test]
fn a_requested_token_is_accepted_by_openssl_ts_verify() {
    let fixture = TsaFixture::build();
    let digest = sample_digest();

    let token = fixture.adapter().request(&digest).unwrap();
    assert!(!token.is_empty(), "the token must carry DER bytes");

    // Independent check: the openssl tool itself must accept the token
    // for this digest against the internal root certificate.
    let token_path = fixture.scratch_path("issued.tsr");
    fs::write(&token_path, &token).unwrap();
    let output = Command::new("openssl")
        .args(["ts", "-verify", "-digest", &digest.to_hex(), "-in"])
        .arg(&token_path)
        .arg("-CAfile")
        .arg(fixture.root_certificate_path())
        .output()
        .expect("openssl is runnable");
    assert!(
        output.status.success(),
        "openssl ts -verify rejected the token: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn consecutive_requests_yield_distinct_tokens() {
    let fixture = TsaFixture::build();
    let adapter = fixture.adapter();
    let digest = sample_digest();

    let first = adapter.request(&digest).unwrap();
    let second = adapter.request(&digest).unwrap();
    // Serial numbers and generation times move forward, so two tokens
    // over the same digest never share bytes.
    assert_ne!(first, second);
}

#[test]
fn request_leaves_no_scratch_files_in_the_tsa_directory() {
    let fixture = TsaFixture::build();
    fixture.adapter().request(&sample_digest()).unwrap();

    let leftovers: Vec<String> = fs::read_dir(&fixture.tsa_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tsq") || name.ends_with(".tsr"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "request and reply files must be cleaned up, found: {leftovers:?}"
    );
}

#[test]
fn a_missing_tsa_directory_is_a_clear_authority_failure() {
    let dir = tempfile::tempdir().unwrap();
    let adapter = LocalOpensslTsa::new(pki_dir().join("tsa.cnf"), dir.path().join("absent"));

    let err = adapter.request(&sample_digest()).unwrap_err();
    match err {
        DomainError::TimestampAuthorityFailure(message) => {
            assert!(
                message.contains("TSA working directory"),
                "the failure should name the missing directory, got: {message}"
            );
        }
        other => panic!("expected a timestamp authority failure, got {other:?}"),
    }
}

#[test]
fn a_requested_token_verifies_as_valid_with_a_plausible_time() {
    let fixture = TsaFixture::build();
    let digest = sample_digest();
    let token = fixture.adapter().request(&digest).unwrap();
    let anchor = fs::read(fixture.root_certificate_path()).unwrap();

    let outcome = Rfc3161Verifier::new()
        .verify(&token, &digest, &anchor)
        .unwrap();
    match outcome {
        TimestampVerification::Valid { generated_at } => {
            // A plausible generation time is RFC 3339 UTC text dated on
            // or after the year this test suite was written.
            assert!(generated_at.ends_with('Z'), "utc time: {generated_at}");
            let year: i32 = generated_at[..4].parse().unwrap();
            assert!(year >= 2026, "implausible year in {generated_at}");
        }
        other => panic!("expected a valid outcome, got {other:?}"),
    }
}

#[test]
fn a_bare_token_without_the_response_wrapper_also_verifies() {
    let fixture = TsaFixture::build();
    let digest = sample_digest();
    let token = fixture.adapter().request(&digest).unwrap();
    let anchor = fs::read(fixture.root_certificate_path()).unwrap();

    // Strip the response wrapper with the openssl tool, leaving the CMS
    // token alone.
    let response_path = fixture.scratch_path("full.tsr");
    let bare_path = fixture.scratch_path("bare.der");
    fs::write(&response_path, &token).unwrap();
    let output = Command::new("openssl")
        .args(["ts", "-reply", "-in"])
        .arg(&response_path)
        .arg("-token_out")
        .arg("-out")
        .arg(&bare_path)
        .output()
        .expect("openssl is runnable");
    assert!(output.status.success());
    let bare = fs::read(&bare_path).unwrap();

    let outcome = Rfc3161Verifier::new()
        .verify(&bare, &digest, &anchor)
        .unwrap();
    assert!(
        matches!(outcome, TimestampVerification::Valid { .. }),
        "expected a valid outcome, got {outcome:?}"
    );
}

#[test]
fn a_token_for_another_digest_is_an_imprint_mismatch_before_any_subprocess() {
    let fixture = TsaFixture::build();
    let token = fixture.adapter().request(&sample_digest()).unwrap();
    let other = RingSha256Hasher::new().hash_bytes(b"otro expediente");

    // The unusable empty anchor proves the mismatch is detected in the
    // native parsing stage: the subprocess stage could not succeed with
    // it and would report UntrustedToken instead.
    let outcome = Rfc3161Verifier::new().verify(&token, &other, b"").unwrap();
    assert_eq!(outcome, TimestampVerification::ImprintMismatch);
}

#[test]
fn random_bytes_are_a_malformed_token() {
    let outcome = Rfc3161Verifier::new()
        .verify(&[0x42u8; 96], &sample_digest(), b"")
        .unwrap();
    assert!(
        matches!(outcome, TimestampVerification::MalformedToken(_)),
        "expected a malformed outcome, got {outcome:?}"
    );
}

#[test]
fn a_token_checked_against_an_unrelated_anchor_is_untrusted() {
    let fixture = TsaFixture::build();
    let digest = sample_digest();
    let token = fixture.adapter().request(&digest).unwrap();

    // An unrelated self-signed certificate as the trust anchor: the
    // token parses and the imprint matches, but the chain must fail.
    let unrelated = tempfile::tempdir().unwrap();
    let key_path = unrelated.path().join("other.key");
    let cert_path = unrelated.path().join("other.crt");
    let output = Command::new("openssl")
        .args([
            "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "2",
        ])
        .arg("-subj")
        .arg("/CN=Unrelated Anchor")
        .arg("-keyout")
        .arg(&key_path)
        .arg("-out")
        .arg(&cert_path)
        .output()
        .expect("openssl is runnable");
    assert!(output.status.success());
    let anchor = fs::read(&cert_path).unwrap();

    let outcome = Rfc3161Verifier::new()
        .verify(&token, &digest, &anchor)
        .unwrap();
    match outcome {
        TimestampVerification::UntrustedToken(detail) => {
            assert!(!detail.is_empty(), "the openssl diagnostic must be kept");
        }
        other => panic!("expected an untrusted outcome, got {other:?}"),
    }
}

#[test]
fn token_info_reports_the_imprint_and_generation_time() {
    let fixture = TsaFixture::build();
    let digest = sample_digest();
    let token = fixture.adapter().request(&digest).unwrap();

    let info = token_info(&token).unwrap();
    assert_eq!(info.digest_hex, digest.to_hex());
    assert!(
        info.generated_at.ends_with('Z'),
        "utc: {}",
        info.generated_at
    );
}

#[test]
fn an_uninitialized_tsa_directory_surfaces_the_reply_failure() {
    let dir = tempfile::tempdir().unwrap();
    let empty = dir.path().join("tsa");
    fs::create_dir_all(&empty).unwrap();
    let adapter = LocalOpensslTsa::new(pki_dir().join("tsa.cnf"), &empty);

    let err = adapter.request(&sample_digest()).unwrap_err();
    match err {
        DomainError::TimestampAuthorityFailure(message) => {
            assert!(
                message.contains("ts -reply"),
                "the failure should name the failing step, got: {message}"
            );
        }
        other => panic!("expected a timestamp authority failure, got {other:?}"),
    }
}
