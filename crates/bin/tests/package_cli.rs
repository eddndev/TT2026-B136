//! End-to-end checks of `package export` through the compiled binary:
//! the produced archive is extracted with the system unzip binary and
//! then verified by running exactly the openssl commands the generated
//! INSTRUCCIONES.md documents, including one corruption case that those
//! commands must detect.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Evidence inputs prepared once: authority, signer, timestamp
/// authority, and a signed and timestamped document.
struct Fixture {
    dir: tempfile::TempDir,
    document: PathBuf,
    signature: PathBuf,
    token: PathBuf,
    certificate: PathBuf,
    root: PathBuf,
    crl: PathBuf,
    tsa_chain: PathBuf,
}

fn pki_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

fn exe() -> &'static str {
    env!("CARGO_BIN_EXE_despacho-cli")
}

fn run_checked(command: &mut Command, when: &str) -> Output {
    let output = command.output().expect("the process runs");
    assert!(
        output.status.success(),
        "{when} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

impl Fixture {
    fn build() -> Self {
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
    fn export(&self, out: &Path, json: bool) -> Output {
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
    fn export_and_extract(&self) -> PathBuf {
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

/// Runs one shell command inside `dir`, mirroring how a third party
/// would follow the instructions, and returns its output.
fn documented_step(dir: &Path, script: &str) -> Output {
    Command::new("bash")
        .arg("-ec")
        .arg(script)
        .current_dir(dir)
        .output()
        .expect("bash is runnable")
}

fn assert_step_ok(dir: &Path, script: &str) -> String {
    let output = documented_step(dir, script);
    assert!(
        output.status.success(),
        "documented command failed: {script}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn the_documented_openssl_commands_verify_the_extracted_package() {
    let fixture = Fixture::build();
    let extracted = fixture.export_and_extract();

    let instructions = fs::read_to_string(extracted.join("INSTRUCCIONES.md"))
        .expect("the package carries INSTRUCCIONES.md");

    // Integrity: the digest the instructions record matches a fresh
    // openssl computation over the extracted document.
    let digest_line = assert_step_ok(&extracted, "openssl dgst -sha256 acta.txt");
    let fresh_digest = digest_line
        .rsplit(' ')
        .next()
        .expect("openssl dgst prints the digest last")
        .trim()
        .to_string();
    assert!(
        instructions.contains(&fresh_digest),
        "the instructions must record the digest {fresh_digest}"
    );

    // Signature: exactly the two documented commands.
    let verified = assert_step_ok(
        &extracted,
        "openssl x509 -in certificado.pem -pubkey -noout > firmante.pub.pem && \
         openssl dgst -sha256 -verify firmante.pub.pem -signature acta.txt.sig acta.txt",
    );
    assert!(
        verified.contains("Verified OK"),
        "openssl must accept the signature: {verified}"
    );

    // Certificate chain and revocation state.
    let chain = assert_step_ok(
        &extracted,
        "cat ca.pem crl.pem > ca-y-crl.pem && \
         openssl verify -crl_check -CAfile ca-y-crl.pem certificado.pem",
    );
    assert!(
        chain.contains("certificado.pem: OK"),
        "openssl must accept the certificate: {chain}"
    );

    // Timestamp token over the document.
    assert_step_ok(
        &extracted,
        "openssl ts -verify -data acta.txt -in acta.txt.tsr -CAfile ca.pem",
    );
}

#[test]
fn the_package_members_survive_the_round_trip_byte_for_byte() {
    let fixture = Fixture::build();
    let extracted = fixture.export_and_extract();

    let expectations = [
        ("acta.txt", &fixture.document),
        ("acta.txt.sig", &fixture.signature),
        ("acta.txt.tsr", &fixture.token),
        ("certificado.pem", &fixture.certificate),
        ("ca.pem", &fixture.root),
        ("crl.pem", &fixture.crl),
        ("tsa-chain.pem", &fixture.tsa_chain),
    ];
    for (member, original) in expectations {
        assert_eq!(
            fs::read(extracted.join(member)).expect(member),
            fs::read(original).unwrap(),
            "{member} must match its source byte for byte"
        );
    }
}

#[test]
fn a_corrupted_member_is_caught_by_the_documented_commands() {
    let fixture = Fixture::build();
    let extracted = fixture.export_and_extract();

    // Corrupt the extracted document the way a post-extraction tamper
    // would: one flipped byte.
    let member = extracted.join("acta.txt");
    let mut bytes = fs::read(&member).unwrap();
    bytes[5] ^= 0xff;
    fs::write(&member, bytes).unwrap();

    let signature_check = documented_step(
        &extracted,
        "openssl x509 -in certificado.pem -pubkey -noout > firmante.pub.pem && \
         openssl dgst -sha256 -verify firmante.pub.pem -signature acta.txt.sig acta.txt",
    );
    assert!(
        !signature_check.status.success(),
        "the documented signature check must reject the altered document"
    );

    let token_check = documented_step(
        &extracted,
        "openssl ts -verify -data acta.txt -in acta.txt.tsr -CAfile ca.pem",
    );
    assert!(
        !token_check.status.success(),
        "the documented token check must reject the altered document"
    );
}

#[test]
fn export_reports_the_digest_and_package_path_as_json() {
    let fixture = Fixture::build();
    let package = fixture.dir.path().join("evidencia.zip");
    let output = fixture.export(&package, true);
    assert!(
        output.status.success(),
        "package export --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
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
    assert_eq!(body["package"], package.display().to_string());
    assert_eq!(body["instructions"], "INSTRUCCIONES.md");
    assert!(package.is_file(), "the archive must exist at --out");
}
