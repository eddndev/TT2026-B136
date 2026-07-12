//! Shared end-to-end fixture for the verification tests: an
//! initialized authority, an issued signer, a prepared timestamp
//! authority, and a document signed and timestamped through the
//! compiled binary, all inside one temporary directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub ca_dir: PathBuf,
    pub tsa_dir: PathBuf,
    pub document: PathBuf,
    pub signature: PathBuf,
    pub token: PathBuf,
    pub certificate: PathBuf,
    pub root: PathBuf,
    pub crl: PathBuf,
    pub serial: String,
}

pub fn pki_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

pub fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_despacho-cli")
}

pub fn run_checked(command: &mut Command, when: &str) -> Output {
    let output = command.output().expect("the process runs");
    assert!(
        output.status.success(),
        "{when} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

impl Fixture {
    pub fn build() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ca_dir = dir.path().join("ca");
        let tsa_dir = dir.path().join("tsa");

        run_checked(
            Command::new(exe())
                .args(["pki", "--scripts-dir"])
                .arg(pki_dir())
                .arg("init-ca")
                .env("PKI_CA_DIR", &ca_dir),
            "pki init-ca",
        );
        let issue = run_checked(
            Command::new(exe())
                .args(["pki", "--scripts-dir"])
                .arg(pki_dir())
                .args(["issue", "--cn", "Firma Prueba", "--json"])
                .env("PKI_CA_DIR", &ca_dir),
            "pki issue",
        );
        let issued: serde_json::Value =
            serde_json::from_slice(&issue.stdout).expect("pki issue --json prints one object");
        let certificate = PathBuf::from(issued["certificate"].as_str().unwrap());
        let key = PathBuf::from(issued["private_key"].as_str().unwrap());
        let serial = issued["serial"].as_str().unwrap().to_string();

        run_checked(
            Command::new("bash")
                .arg(pki_dir().join("issue-tsa-cert.sh"))
                .env("PKI_CA_DIR", &ca_dir)
                .env("TSA_DIR", &tsa_dir),
            "issue-tsa-cert.sh",
        );
        run_checked(
            Command::new(exe())
                .args(["pki", "--scripts-dir"])
                .arg(pki_dir())
                .arg("gen-crl")
                .env("PKI_CA_DIR", &ca_dir),
            "pki gen-crl",
        );

        let document = dir.path().join("acta.txt");
        fs::write(&document, b"acta de la audiencia del doce de julio\n").unwrap();
        run_checked(
            Command::new(exe())
                .arg("sign")
                .arg(&document)
                .arg("--cert")
                .arg(&certificate)
                .arg("--key")
                .arg(&key),
            "sign",
        );
        run_checked(
            Command::new(exe())
                .arg("timestamp")
                .arg(&document)
                .arg("--mock")
                .arg("--pki-dir")
                .arg(pki_dir())
                .env("TSA_DIR", &tsa_dir),
            "timestamp --mock",
        );

        let signature = path_with_suffix(&document, ".sig");
        let token = path_with_suffix(&document, ".tsr");
        let root = ca_dir.join("ca.crt.pem");
        let crl = ca_dir.join("crl/crl.pem");
        Self {
            dir,
            ca_dir,
            tsa_dir,
            document,
            signature,
            token,
            certificate,
            root,
            crl,
            serial,
        }
    }

    /// Runs `verify` over `document` with the fixture's signature,
    /// certificate, and root, plus `extra` arguments; the timestamp
    /// token travels along when `with_token` is set.
    pub fn verify(&self, document: &Path, with_token: bool, extra: &[&str]) -> Output {
        let mut command = Command::new(exe());
        command
            .arg("verify")
            .arg(document)
            .arg("--sig")
            .arg(&self.signature)
            .arg("--cert")
            .arg(&self.certificate)
            .arg("--ca")
            .arg(&self.root);
        if with_token {
            command.arg("--tsr").arg(&self.token);
        }
        command.args(extra);
        command.output().expect("the binary runs")
    }
}

pub fn path_with_suffix(file: &Path, suffix: &str) -> PathBuf {
    let mut name = file.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

pub fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}
