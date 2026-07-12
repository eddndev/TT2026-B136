//! Shared end-to-end fixture for the evidence-package tests: an
//! initialized authority, an issued signer, a prepared timestamp
//! authority, and a document signed and timestamped through the
//! compiled binary, all inside one temporary directory.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Evidence inputs prepared once: authority, signer, timestamp
/// authority, and a signed and timestamped document.
pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub document: PathBuf,
    pub signature: PathBuf,
    pub token: PathBuf,
    pub certificate: PathBuf,
    pub root: PathBuf,
    pub crl: PathBuf,
    pub tsa_chain: PathBuf,
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
                .args(["issue", "--cn", "Evidencia Prueba", "--json"])
                .env("PKI_CA_DIR", &ca_dir),
            "pki issue",
        );
        let issued: serde_json::Value =
            serde_json::from_slice(&issue.stdout).expect("pki issue --json prints one object");
        let certificate = PathBuf::from(issued["certificate"].as_str().unwrap());
        let key = PathBuf::from(issued["private_key"].as_str().unwrap());

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
        fs::write(&document, b"acta que la evidencia debe preservar\n").unwrap();
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

        let signature = PathBuf::from(format!("{}.sig", document.display()));
        let token = PathBuf::from(format!("{}.tsr", document.display()));
        let root = ca_dir.join("ca.crt.pem");
        let crl = ca_dir.join("crl/crl.pem");
        let tsa_chain = tsa_dir.join("tsa-chain.pem");
        Self {
            dir,
            document,
            signature,
            token,
            certificate,
            root,
            crl,
            tsa_chain,
        }
    }

    /// Exports the package to `out` and returns the command output.
    pub fn export(&self, out: &Path, json: bool) -> Output {
        let mut command = Command::new(exe());
        command
            .args(["package", "export"])
            .arg(&self.document)
            .arg("--sig")
            .arg(&self.signature)
            .arg("--tsr")
            .arg(&self.token)
            .arg("--cert")
            .arg(&self.certificate)
            .arg("--ca")
            .arg(&self.root)
            .arg("--crl")
            .arg(&self.crl)
            .arg("--tsa-chain")
            .arg(&self.tsa_chain)
            .arg("--out")
            .arg(out);
        if json {
            command.arg("--json");
        }
        command.output().expect("the binary runs")
    }

    /// Exports and extracts the package, returning the extraction dir.
    pub fn export_and_extract(&self) -> PathBuf {
        let package = self.dir.path().join("evidencia.zip");
        let output = self.export(&package, false);
        assert!(
            output.status.success(),
            "package export failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let unzip_test = Command::new("unzip")
            .arg("-t")
            .arg(&package)
            .output()
            .expect("unzip is runnable");
        assert!(
            unzip_test.status.success(),
            "unzip -t rejected the package: {}",
            String::from_utf8_lossy(&unzip_test.stdout)
        );

        let extracted = self.dir.path().join("extraccion");
        run_checked(
            Command::new("unzip")
                .arg("-o")
                .arg(&package)
                .arg("-d")
                .arg(&extracted),
            "unzip extraction",
        );
        extracted
    }
}
