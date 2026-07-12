//! End-to-end checks of `sign` through the compiled binary, using keys and
//! certificates issued in-test with the openssl command line.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A per-test scratch directory, removed when the value drops.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> Self {
        let mut dir = std::env::temp_dir();
        dir.push(format!("despacho-sign-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("scratch dir is creatable");
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn openssl(args: &[&str], when: &str) -> Output {
    let output = Command::new("openssl")
        .args(args)
        .output()
        .expect("openssl must be runnable");
    assert!(
        output.status.success(),
        "{when} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Issues a fresh RSA-3072 key and a self-signed certificate for `cn`.
fn issue_credentials(dir: &ScratchDir, prefix: &str, cn: &str) -> (PathBuf, PathBuf) {
    let key = dir.path(&format!("{prefix}.key.pem"));
    let cert = dir.path(&format!("{prefix}.crt.pem"));
    openssl(
        &[
            "genpkey",
            "-quiet",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:3072",
            "-out",
            key.to_str().unwrap(),
        ],
        "key generation",
    );
    let subject = format!("/CN={cn}");
    openssl(
        &[
            "req",
            "-x509",
            "-key",
            key.to_str().unwrap(),
            "-subj",
            &subject,
            "-days",
            "1",
            "-sha256",
            "-out",
            cert.to_str().unwrap(),
        ],
        "certificate issuance",
    );
    (key, cert)
}

fn write_sample_document(dir: &ScratchDir) -> PathBuf {
    let path = dir.path("document.bin");
    let data: Vec<u8> = (0..2048u32).map(|i| (i % 249) as u8).collect();
    fs::write(&path, data).expect("document is writable");
    path
}

fn run_sign(file: &Path, cert: &Path, key: &Path, json: bool) -> Output {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let mut cmd = Command::new(exe);
    cmd.arg("sign")
        .arg(file)
        .arg("--cert")
        .arg(cert)
        .arg("--key")
        .arg(key);
    if json {
        cmd.arg("--json");
    }
    cmd.output().expect("the binary runs")
}

fn sha256sum_of(path: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(path)
        .output()
        .expect("sha256sum must be runnable");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

#[test]
fn sign_writes_a_signature_that_openssl_verifies() {
    let dir = ScratchDir::new("roundtrip");
    let (key, cert) = issue_credentials(&dir, "signer", "Cli Signer");
    let document = write_sample_document(&dir);

    let output = run_sign(&document, &cert, &key, false);
    assert!(
        output.status.success(),
        "sign failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The digest printed is the document's SHA-256.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&sha256sum_of(&document)),
        "stdout must carry the digest hex, got: {stdout}"
    );

    // The signature lands next to the document and openssl accepts it.
    let sig = dir.path("document.bin.sig");
    assert!(sig.is_file(), "sign writes <file>.sig");
    assert!(
        stdout.contains("document.bin.sig"),
        "stdout must name the signature path, got: {stdout}"
    );
    let public_key = dir.path("signer.pub.pem");
    openssl(
        &[
            "x509",
            "-in",
            cert.to_str().unwrap(),
            "-pubkey",
            "-noout",
            "-out",
            public_key.to_str().unwrap(),
        ],
        "public key extraction",
    );
    let verified = openssl(
        &[
            "dgst",
            "-sha256",
            "-verify",
            public_key.to_str().unwrap(),
            "-signature",
            sig.to_str().unwrap(),
            document.to_str().unwrap(),
        ],
        "openssl verification",
    );
    assert!(String::from_utf8_lossy(&verified.stdout).contains("Verified OK"));
}

#[test]
fn sign_reports_json_with_digest_path_and_subject() {
    let dir = ScratchDir::new("json");
    let (key, cert) = issue_credentials(&dir, "signer", "Json Signer");
    let document = write_sample_document(&dir);

    let output = run_sign(&document, &cert, &key, true);
    assert!(
        output.status.success(),
        "sign --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is one json object");
    assert_eq!(
        body["digest"].as_str().unwrap(),
        sha256sum_of(&document),
        "digest field must be the document sha-256"
    );
    let signature_path = PathBuf::from(body["signature_path"].as_str().unwrap());
    assert!(signature_path.is_file());
    assert!(body["certificate_subject"]
        .as_str()
        .unwrap()
        .contains("Json Signer"));
}

#[test]
fn sign_rejects_a_key_that_does_not_match_the_certificate() {
    let dir = ScratchDir::new("mismatch");
    let (key, _) = issue_credentials(&dir, "signer", "Real Signer");
    let (_, other_cert) = issue_credentials(&dir, "other", "Other Signer");
    let document = write_sample_document(&dir);

    let output = run_sign(&document, &other_cert, &key, false);
    assert!(
        !output.status.success(),
        "a mismatched key and certificate must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("does not match the certificate"),
        "stderr must explain the mismatch, got: {stderr}"
    );
    assert!(
        !dir.path("document.bin.sig").exists(),
        "no signature file may be written on a mismatch"
    );
}

#[test]
fn sign_rejects_an_undersized_key() {
    let dir = ScratchDir::new("small-key");
    let key = dir.path("small.key.pem");
    openssl(
        &[
            "genpkey",
            "-quiet",
            "-algorithm",
            "RSA",
            "-pkeyopt",
            "rsa_keygen_bits:2048",
            "-out",
            key.to_str().unwrap(),
        ],
        "key generation",
    );
    let cert = dir.path("small.crt.pem");
    openssl(
        &[
            "req",
            "-x509",
            "-key",
            key.to_str().unwrap(),
            "-subj",
            "/CN=Small Key",
            "-days",
            "1",
            "-sha256",
            "-out",
            cert.to_str().unwrap(),
        ],
        "certificate issuance",
    );
    let document = write_sample_document(&dir);

    let output = run_sign(&document, &cert, &key, false);
    assert!(!output.status.success(), "a 2048-bit key must be rejected");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("3072"),
        "stderr must name the required modulus size, got: {stderr}"
    );
}
