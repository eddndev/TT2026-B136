//! Chain-validation tests against fixtures produced by the versioned
//! OpenSSL authority scripts under `pki/`, plus an interoperability check
//! that `openssl verify -crl_check` agrees with the validator.
//!
//! Fixtures are generated once per test run inside a temporary directory:
//! a main authority with one valid and one revoked certificate plus a
//! fresh revocation list, and a second, independent authority whose
//! certificates must not chain to the main root. Evaluation times derive
//! from the parsed validity window, never from sleeping.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use domain::crypto::certificate::{CertificateValidation, CertificateValidator};
use domain::DomainError;
use infrastructure::X509ChainValidator;

struct Fixture {
    /// Owns the directory holding both authorities; dropped never because
    /// the fixture lives in a static.
    _dir: tempfile::TempDir,
    root_pem: Vec<u8>,
    valid_cert_pem: Vec<u8>,
    valid_cert_path: PathBuf,
    valid_cert_der: Vec<u8>,
    revoked_cert_pem: Vec<u8>,
    revoked_cert_path: PathBuf,
    revoked_serial: String,
    crl_pem: Vec<u8>,
    foreign_cert_pem: Vec<u8>,
    /// Root certificate and revocation list concatenated, as
    /// `openssl verify -crl_check -CAfile` expects.
    ca_bundle_path: PathBuf,
    not_before: i64,
    not_after: i64,
}

fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

fn run_script(name: &str, ca_dir: &Path, args: &[&str]) {
    let output = Command::new("bash")
        .arg(scripts_dir().join(name))
        .args(args)
        .env("PKI_CA_DIR", ca_dir)
        .output()
        .unwrap_or_else(|err| panic!("cannot run {name}: {err}"));
    assert!(
        output.status.success(),
        "{name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn pem_to_der(pem_path: &Path) -> Vec<u8> {
    let output = Command::new("openssl")
        .args(["x509", "-outform", "DER", "-in"])
        .arg(pem_path)
        .output()
        .expect("openssl must run");
    assert!(output.status.success(), "openssl x509 -outform DER failed");
    output.stdout
}

fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = tempfile::tempdir().expect("tempdir is creatable");
        let ca_dir = dir.path().join("main-ca");
        run_script("init-ca.sh", &ca_dir, &[]);
        run_script("issue-cert.sh", &ca_dir, &["Firmante Vigente"]);
        run_script("issue-cert.sh", &ca_dir, &["Firmante Revocado"]);
        let revoked_cert_path = ca_dir.join("certs/firmante-revocado.crt.pem");
        run_script("revoke.sh", &ca_dir, &[revoked_cert_path.to_str().unwrap()]);
        run_script("gen-crl.sh", &ca_dir, &[]);

        let foreign_ca_dir = dir.path().join("foreign-ca");
        run_script("init-ca.sh", &foreign_ca_dir, &[]);
        run_script("issue-cert.sh", &foreign_ca_dir, &["Firmante Ajeno"]);

        let root_pem = fs::read(ca_dir.join("ca.crt.pem")).unwrap();
        let crl_pem = fs::read(ca_dir.join("crl/crl.pem")).unwrap();
        let ca_bundle_path = dir.path().join("ca-bundle.pem");
        fs::write(&ca_bundle_path, [root_pem.as_slice(), &crl_pem].concat()).unwrap();

        let valid_cert_path = ca_dir.join("certs/firmante-vigente.crt.pem");
        let valid_cert_pem = fs::read(&valid_cert_path).unwrap();
        let revoked_cert_pem = fs::read(&revoked_cert_path).unwrap();

        let validator = X509ChainValidator::new();
        let summary = validator.inspect(&valid_cert_pem).unwrap();
        let revoked_serial = validator.inspect(&revoked_cert_pem).unwrap().serial_hex;

        Fixture {
            valid_cert_der: pem_to_der(&valid_cert_path),
            foreign_cert_pem: fs::read(foreign_ca_dir.join("certs/firmante-ajeno.crt.pem"))
                .unwrap(),
            _dir: dir,
            root_pem,
            valid_cert_pem,
            valid_cert_path,
            revoked_cert_pem,
            revoked_cert_path,
            revoked_serial,
            crl_pem,
            ca_bundle_path,
            not_before: summary.not_before_unix,
            not_after: summary.not_after_unix,
        }
    })
}

/// One hour into the validity window: the certificate is current and the
/// seven-day revocation list is still fresh.
fn inside_window(fx: &Fixture) -> i64 {
    fx.not_before + 3_600
}

#[test]
fn issued_certificate_is_valid_inside_its_window_with_a_fresh_crl() {
    let fx = fixture();
    let outcome = X509ChainValidator::new()
        .validate(
            &fx.valid_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            inside_window(fx),
        )
        .unwrap();
    assert_eq!(outcome, CertificateValidation::Valid);
}

#[test]
fn evaluation_after_not_after_reports_expired() {
    let fx = fixture();
    // The validity window is checked before revocation, so the verdict is
    // expired even though the revocation list is also stale by then.
    let outcome = X509ChainValidator::new()
        .validate(
            &fx.valid_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            fx.not_after + 60,
        )
        .unwrap();
    assert_eq!(outcome, CertificateValidation::Expired);
}

#[test]
fn evaluation_before_not_before_reports_not_yet_valid() {
    let fx = fixture();
    let outcome = X509ChainValidator::new()
        .validate(
            &fx.valid_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            fx.not_before - 3_600,
        )
        .unwrap();
    assert_eq!(outcome, CertificateValidation::NotYetValid);
}

#[test]
fn revoked_certificate_is_reported_with_its_serial() {
    let fx = fixture();
    let outcome = X509ChainValidator::new()
        .validate(
            &fx.revoked_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            inside_window(fx),
        )
        .unwrap();
    assert_eq!(
        outcome,
        CertificateValidation::Revoked {
            serial_hex: fx.revoked_serial.clone(),
        }
    );
}

#[test]
fn certificate_from_an_independent_authority_is_untrusted() {
    let fx = fixture();
    // Both authorities share the same subject name (it is fixed in
    // pki/openssl.cnf), so only the signature check can tell them apart.
    let outcome = X509ChainValidator::new()
        .validate(
            &fx.foreign_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            inside_window(fx),
        )
        .unwrap();
    assert_eq!(outcome, CertificateValidation::UntrustedIssuer);
}

#[test]
fn crl_past_its_next_update_is_a_stale_crl_error() {
    let fx = fixture();
    // Eight days in: the certificate is still current but the seven-day
    // revocation list has missed its scheduled update.
    let eight_days_in = fx.not_before + 8 * 86_400;
    let err = X509ChainValidator::new()
        .validate(
            &fx.valid_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            eight_days_in,
        )
        .unwrap_err();
    match err {
        DomainError::StaleCrl { next_update_unix } => {
            assert!(
                next_update_unix < eight_days_in,
                "the missed update time must precede the evaluation time"
            );
        }
        other => panic!("expected a stale crl error, got {other:?}"),
    }
}

#[test]
fn corrupting_one_signature_byte_makes_the_issuer_untrusted() {
    let fx = fixture();
    // The signature is the last field of the DER encoding, so flipping the
    // final byte corrupts it while the certificate still parses.
    let mut der_bytes = fx.valid_cert_der.clone();
    let last = der_bytes.len() - 1;
    der_bytes[last] ^= 0x01;
    let outcome = X509ChainValidator::new()
        .validate(&der_bytes, &fx.root_pem, None, inside_window(fx))
        .unwrap();
    assert_eq!(outcome, CertificateValidation::UntrustedIssuer);
}

#[test]
fn der_input_is_accepted_alongside_pem() {
    let fx = fixture();
    let outcome = X509ChainValidator::new()
        .validate(&fx.valid_cert_der, &fx.root_pem, None, inside_window(fx))
        .unwrap();
    assert_eq!(outcome, CertificateValidation::Valid);
}

#[test]
fn inspect_reports_subject_issuer_serial_and_window() {
    let fx = fixture();
    let summary = X509ChainValidator::new()
        .inspect(&fx.valid_cert_pem)
        .unwrap();
    assert!(
        summary.subject.contains("Firmante Vigente"),
        "subject should carry the common name, got: {}",
        summary.subject
    );
    assert!(
        summary.issuer.contains("CA Raiz Interna"),
        "issuer should carry the root common name, got: {}",
        summary.issuer
    );
    assert_eq!(summary.serial_hex, "1000");
    assert!(summary.not_before_unix < summary.not_after_unix);
}

/// `openssl verify -crl_check` evaluates at the current system time; the
/// fixtures were created moments ago, so the validator is queried inside
/// the window where both must agree.
#[test]
fn openssl_verify_agrees_on_the_valid_and_revoked_cases() {
    let fx = fixture();
    let validator = X509ChainValidator::new();

    let valid_verify = openssl_verify(&fx.ca_bundle_path, &fx.valid_cert_path);
    assert!(
        valid_verify.status.success(),
        "openssl must accept the valid certificate"
    );
    let outcome = validator
        .validate(
            &fx.valid_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            inside_window(fx),
        )
        .unwrap();
    assert_eq!(outcome, CertificateValidation::Valid);

    let revoked_verify = openssl_verify(&fx.ca_bundle_path, &fx.revoked_cert_path);
    assert!(
        !revoked_verify.status.success(),
        "openssl must reject the revoked certificate"
    );
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&revoked_verify.stdout),
        String::from_utf8_lossy(&revoked_verify.stderr)
    );
    assert!(
        text.contains("certificate revoked"),
        "openssl should name the revocation, got: {text}"
    );
    let outcome = validator
        .validate(
            &fx.revoked_cert_pem,
            &fx.root_pem,
            Some(&fx.crl_pem),
            inside_window(fx),
        )
        .unwrap();
    assert_eq!(
        outcome,
        CertificateValidation::Revoked {
            serial_hex: fx.revoked_serial.clone(),
        }
    );
}

fn openssl_verify(ca_bundle: &Path, cert: &Path) -> std::process::Output {
    Command::new("openssl")
        .args(["verify", "-crl_check", "-CAfile"])
        .arg(ca_bundle)
        .arg(cert)
        .output()
        .expect("openssl must run")
}
