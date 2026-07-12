//! End-to-end checks of the `timestamp` command through the compiled
//! binary, driving the local authority prepared with the pki scripts
//! in a temporary directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use domain::crypto::DocumentHasher;
use infrastructure::RingSha256Hasher;

fn pki_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

/// An initialized CA plus an issued TSA working directory inside one
/// temporary directory.
struct TsaFixture {
    dir: tempfile::TempDir,
    tsa_dir: PathBuf,
}

impl TsaFixture {
    fn build() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ca_dir = dir.path().join("ca");
        let tsa_dir = dir.path().join("tsa");
        for script in ["init-ca.sh", "issue-tsa-cert.sh"] {
            let output = Command::new("bash")
                .arg(pki_dir().join(script))
                .env("PKI_CA_DIR", &ca_dir)
                .env("TSA_DIR", &tsa_dir)
                .output()
                .expect("bash and the pki scripts are runnable");
            assert!(
                output.status.success(),
                "{script} failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Self { dir, tsa_dir }
    }

    fn write_input(&self, name: &str, content: &[u8]) -> PathBuf {
        let path = self.dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn root_certificate_path(&self) -> PathBuf {
        self.dir.path().join("ca").join("ca.crt.pem")
    }
}

fn run_timestamp(fixture: &TsaFixture, args: &[&str]) -> Output {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let mut cmd = Command::new(exe);
    cmd.arg("timestamp").args(args);
    cmd.arg("--pki-dir").arg(pki_dir());
    cmd.env("TSA_DIR", &fixture.tsa_dir);
    cmd.output().expect("the binary runs")
}

#[test]
fn timestamp_with_mock_writes_a_verifiable_token_next_to_the_input() {
    let fixture = TsaFixture::build();
    let content = b"acta de audiencia";
    let input = fixture.write_input("acta.txt", content);

    let output = run_timestamp(&fixture, &[input.to_str().unwrap(), "--mock", "--json"]);
    assert!(
        output.status.success(),
        "timestamp --mock failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("--json prints one json object");
    let expected_digest = RingSha256Hasher::new().hash_bytes(content).to_hex();
    assert_eq!(body["digest"], expected_digest.as_str());

    let token_path = body["token_path"].as_str().expect("token path");
    assert_eq!(token_path, format!("{}.tsr", input.display()));
    let token = fs::read(token_path).unwrap();
    assert!(!token.is_empty());

    let generated_at = body["generated_at"].as_str().expect("generation time");
    assert!(
        generated_at.contains('T') && generated_at.ends_with('Z'),
        "rfc 3339 utc time expected, got: {generated_at}"
    );

    // Independent check: openssl accepts the written token for the
    // input's digest against the internal root certificate.
    let verify = Command::new("openssl")
        .args(["ts", "-verify", "-digest", &expected_digest, "-in"])
        .arg(token_path)
        .arg("-CAfile")
        .arg(fixture.root_certificate_path())
        .output()
        .expect("openssl is runnable");
    assert!(
        verify.status.success(),
        "openssl ts -verify rejected the token: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

#[test]
fn timestamp_honors_the_out_flag_and_prints_readable_text() {
    let fixture = TsaFixture::build();
    let input = fixture.write_input("promocion.txt", b"promocion inicial");
    let out = fixture.dir.path().join("sello.tsr");

    let output = run_timestamp(
        &fixture,
        &[
            input.to_str().unwrap(),
            "--mock",
            "--out",
            out.to_str().unwrap(),
        ],
    );
    assert!(
        output.status.success(),
        "timestamp --mock --out failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.is_file(), "the token must land at the --out path");

    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("digest:"), "output: {text}");
    assert!(text.contains(&out.display().to_string()), "output: {text}");
    assert!(text.contains("generated at:"), "output: {text}");
}

#[test]
fn timestamp_without_provider_settings_suggests_the_mock_authority() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("acta.txt");
    fs::write(&input, b"acta").unwrap();

    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let output = Command::new(exe)
        .arg("timestamp")
        .arg(&input)
        // Blank values override anything a local .env could inject.
        .env("CINCEL_BASE_URL", "")
        .env("CINCEL_API_KEY", "")
        .output()
        .expect("the binary runs");

    assert!(
        !output.status.success(),
        "without provider settings the command must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CINCEL_BASE_URL") && stderr.contains("--mock"),
        "stderr should name the missing settings and suggest --mock, got: {stderr}"
    );
}
