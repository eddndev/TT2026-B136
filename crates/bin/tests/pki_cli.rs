//! End-to-end checks of the `pki` command group through the compiled
//! binary, driving the authority scripts against a temporary CA
//! directory supplied via PKI_CA_DIR.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A per-test scratch directory, removed when the value drops.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> Self {
        let mut dir = std::env::temp_dir();
        dir.push(format!("despacho-pki-{tag}-{}", std::process::id()));
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

fn scripts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

fn run_pki(ca_dir: &Path, args: &[&str]) -> Output {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let mut cmd = Command::new(exe);
    cmd.arg("pki").arg("--scripts-dir").arg(scripts_dir());
    cmd.args(args);
    cmd.env("PKI_CA_DIR", ca_dir);
    cmd.output().expect("the binary runs")
}

fn assert_success(output: &Output, when: &str) {
    assert!(
        output.status.success(),
        "{when} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn json_body(output: &Output, when: &str) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!(
            "{when} did not print json ({err}): {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

#[test]
fn init_issue_show_revoke_and_gen_crl_work_end_to_end() {
    let dir = ScratchDir::new("cycle");
    let ca_dir = dir.path("ca");

    let output = run_pki(&ca_dir, &["init-ca"]);
    assert_success(&output, "pki init-ca");
    assert!(ca_dir.join("ca.crt.pem").is_file());

    let output = run_pki(&ca_dir, &["init-ca", "--json"]);
    assert_success(&output, "pki init-ca --json");
    let body = json_body(&output, "pki init-ca --json");
    assert_eq!(body["ca_dir"], ca_dir.display().to_string());

    let output = run_pki(&ca_dir, &["issue", "--cn", "Ana Prueba", "--json"]);
    assert_success(&output, "pki issue");
    let body = json_body(&output, "pki issue --json");
    assert_eq!(body["serial"], "1000");
    let cert_path = body["certificate"].as_str().expect("certificate path");
    assert!(Path::new(cert_path).is_file());
    let key_path = body["private_key"].as_str().expect("private key path");
    assert!(Path::new(key_path).is_file());

    let output = run_pki(&ca_dir, &["show", cert_path]);
    assert_success(&output, "pki show");
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(text.contains("Ana Prueba"), "show output: {text}");
    assert!(text.contains("serial:     1000"), "show output: {text}");

    let output = run_pki(&ca_dir, &["show", cert_path, "--json"]);
    assert_success(&output, "pki show --json");
    let body = json_body(&output, "pki show --json");
    assert!(body["subject"]
        .as_str()
        .expect("subject")
        .contains("Ana Prueba"));
    assert_eq!(body["serial"], "1000");
    let not_before = body["not_before"].as_str().expect("not_before");
    let not_after = body["not_after"].as_str().expect("not_after");
    assert!(not_before.contains('T'), "rfc3339 timestamp: {not_before}");
    assert!(not_before < not_after);

    let output = run_pki(&ca_dir, &["revoke", "--serial", "1000"]);
    assert_success(&output, "pki revoke");

    let output = run_pki(&ca_dir, &["gen-crl", "--json"]);
    assert_success(&output, "pki gen-crl");
    let body = json_body(&output, "pki gen-crl --json");
    let crl_path = body["crl"].as_str().expect("crl path");
    assert!(Path::new(crl_path).is_file());
    let crl = fs::read(crl_path).unwrap();
    assert!(crl.starts_with(b"-----BEGIN X509 CRL-----"));
}

#[test]
fn issuing_before_initializing_the_authority_fails() {
    let dir = ScratchDir::new("no-ca");
    let ca_dir = dir.path("ca");

    let output = run_pki(&ca_dir, &["issue", "--cn", "Sin Autoridad"]);
    assert!(
        !output.status.success(),
        "issuance must fail without an initialized authority"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("certificate issuance failed"),
        "stderr should name the failed step, got: {stderr}"
    );
}

#[test]
fn revoking_an_unknown_serial_fails() {
    let dir = ScratchDir::new("bad-serial");
    let ca_dir = dir.path("ca");

    let output = run_pki(&ca_dir, &["init-ca"]);
    assert_success(&output, "pki init-ca");

    let output = run_pki(&ca_dir, &["revoke", "--serial", "10FF"]);
    assert!(
        !output.status.success(),
        "an unissued serial must not revoke"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no issued certificate with serial 10FF"),
        "stderr should name the unknown serial, got: {stderr}"
    );
}
