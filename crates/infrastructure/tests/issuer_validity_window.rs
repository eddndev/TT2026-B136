//! Interoperability tests for an issuer whose own validity window closes
//! before the certificate it signed: the validator and
//! `openssl verify -attime` must agree on accepting the chain while both
//! windows are open and on rejecting it once the issuer has expired.
//!
//! The versioned authority scripts under `pki/` pin the root lifetime to
//! five years, longer than any certificate they issue, so this fixture
//! builds its own chain with plain openssl commands instead: a self-signed
//! root valid for one day and a leaf valid for one year. Evaluation times
//! derive from the parsed validity windows, never from sleeping.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use domain::crypto::certificate::{CertificateValidation, CertificateValidator};
use infrastructure::X509ChainValidator;

struct Fixture {
    /// Owns the directory holding the keys and certificates; never dropped
    /// because the fixture lives in a static.
    _dir: tempfile::TempDir,
    ca_pem: Vec<u8>,
    ca_path: PathBuf,
    leaf_pem: Vec<u8>,
    leaf_path: PathBuf,
    /// Latest notBefore of the two certificates, unix seconds.
    window_open: i64,
    /// End of the issuer's validity window, unix seconds. The leaf remains
    /// inside its own window for almost a year beyond this.
    ca_not_after: i64,
}

fn openssl(dir: &Path, args: &[&str]) -> std::process::Output {
    let output = Command::new("openssl")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("openssl must run");
    assert!(
        output.status.success(),
        "openssl {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn generate_rsa_key(dir: &Path, name: &str) {
    openssl(
        dir,
        &[
            "genpkey",
            "-quiet",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
            "-out",
            name,
        ],
    );
}

fn fixture() -> &'static Fixture {
    static FIXTURE: OnceLock<Fixture> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let dir = tempfile::tempdir().expect("tempdir is creatable");
        let path = dir.path();

        generate_rsa_key(path, "ca.key.pem");
        openssl(
            path,
            &[
                "req",
                "-x509",
                "-sha256",
                "-days",
                "1",
                "-key",
                "ca.key.pem",
                "-subj",
                "/CN=Raiz Efimera",
                "-out",
                "ca.crt.pem",
            ],
        );

        generate_rsa_key(path, "leaf.key.pem");
        openssl(
            path,
            &[
                "req",
                "-new",
                "-sha256",
                "-key",
                "leaf.key.pem",
                "-subj",
                "/CN=Firmante Duradero",
                "-out",
                "leaf.csr.pem",
            ],
        );
        openssl(
            path,
            &[
                "x509",
                "-req",
                "-sha256",
                "-days",
                "365",
                "-in",
                "leaf.csr.pem",
                "-CA",
                "ca.crt.pem",
                "-CAkey",
                "ca.key.pem",
                "-set_serial",
                "4096",
                "-out",
                "leaf.crt.pem",
            ],
        );

        let ca_path = path.join("ca.crt.pem");
        let leaf_path = path.join("leaf.crt.pem");
        let ca_pem = std::fs::read(&ca_path).unwrap();
        let leaf_pem = std::fs::read(&leaf_path).unwrap();

        let validator = X509ChainValidator::new();
        let ca_summary = validator.inspect(&ca_pem).unwrap();
        let leaf_summary = validator.inspect(&leaf_pem).unwrap();

        Fixture {
            window_open: ca_summary.not_before_unix.max(leaf_summary.not_before_unix),
            ca_not_after: ca_summary.not_after_unix,
            _dir: dir,
            ca_pem,
            ca_path,
            leaf_pem,
            leaf_path,
        }
    })
}

/// Runs `openssl verify -attime <unix_seconds> -CAfile <ca> <leaf>` and
/// returns its combined output and success flag.
fn openssl_verify_at(fx: &Fixture, unix_seconds: i64) -> (bool, String) {
    let output = Command::new("openssl")
        .args(["verify", "-attime", &unix_seconds.to_string(), "-CAfile"])
        .arg(&fx.ca_path)
        .arg(&fx.leaf_path)
        .output()
        .expect("openssl must run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

#[test]
fn chain_is_valid_while_both_windows_are_open_and_openssl_agrees() {
    let fx = fixture();
    let at = fx.window_open + 60;
    let outcome = X509ChainValidator::new()
        .validate(&fx.leaf_pem, &fx.ca_pem, None, at)
        .unwrap();
    assert_eq!(outcome, CertificateValidation::Valid);

    let (accepted, text) = openssl_verify_at(fx, at);
    assert!(accepted, "openssl must accept the chain: {text}");
}

#[test]
fn expired_issuer_is_rejected_and_openssl_agrees() {
    let fx = fixture();
    // Thirty days past the issuer's notAfter: the leaf is still inside its
    // one-year window, so only the issuer's expiry can fail the chain.
    let at = fx.ca_not_after + 30 * 86_400;
    let outcome = X509ChainValidator::new()
        .validate(&fx.leaf_pem, &fx.ca_pem, None, at)
        .unwrap();
    assert_eq!(outcome, CertificateValidation::UntrustedIssuer);

    let (accepted, text) = openssl_verify_at(fx, at);
    assert!(!accepted, "openssl must reject the chain: {text}");
    assert!(
        text.contains("certificate has expired"),
        "openssl should name the issuer expiry, got: {text}"
    );
}
