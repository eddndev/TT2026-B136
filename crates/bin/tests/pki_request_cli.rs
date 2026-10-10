//! Administrative issuance accepts only public requests signed by an external holder.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use base64::{engine::general_purpose::STANDARD, Engine};

const REQUEST_LIMIT: usize = 16 * 1024;
const SUBJECT: &str = "/C=MX/O=Despacho Juridico Demo/CN=Independent Holder";

fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki")
}

fn pki_with_scripts(ca: &Path, directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .arg("pki")
        .arg("--scripts-dir")
        .arg(directory)
        .args(args)
        .env("PKI_CA_DIR", ca)
        .output()
        .unwrap()
}

fn pki(ca: &Path, args: &[&str]) -> Output {
    pki_with_scripts(ca, &scripts(), args)
}

fn success(output: Output) -> Vec<u8> {
    assert!(
        output.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn openssl(args: &[&str]) -> Vec<u8> {
    success(Command::new("openssl").args(args).output().unwrap())
}

fn request(directory: &Path, bits: u32, exponent: u32, digest: &str) -> (PathBuf, PathBuf) {
    fs::create_dir_all(directory).unwrap();
    let key = directory.join("holder.key.pem");
    let csr = directory.join("holder.csr.pem");
    openssl(&[
        "genpkey",
        "-quiet",
        "-algorithm",
        "RSA",
        "-pkeyopt",
        &format!("rsa_keygen_bits:{bits}"),
        "-pkeyopt",
        &format!("rsa_keygen_pubexp:{exponent}"),
        "-out",
        key.to_str().unwrap(),
    ]);
    openssl(&[
        "req",
        "-new",
        digest,
        "-key",
        key.to_str().unwrap(),
        "-subj",
        SUBJECT,
        "-addext",
        "basicConstraints=critical,CA:TRUE",
        "-addext",
        "keyUsage=critical,keyCertSign,cRLSign",
        "-addext",
        "extendedKeyUsage=clientAuth",
        "-addext",
        "subjectAltName=DNS:untrusted.example",
        "-out",
        csr.to_str().unwrap(),
    ]);
    (key, csr)
}

fn issue(ca: &Path, csr: &Path, name: &str) -> Output {
    pki(
        ca,
        &[
            "issue-request",
            "--csr",
            csr.to_str().unwrap(),
            "--name",
            name,
            "--json",
        ],
    )
}

fn state(directory: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.push((
                    path.strip_prefix(root).unwrap().into(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut files = Vec::new();
    visit(directory, directory, &mut files);
    files.sort_by(|left, right| left.0.cmp(&right.0));
    files
}

fn rejected_without_mutation(ca: &Path, csr: &Path, name: &str) {
    let before = state(ca);
    let output = issue(ca, csr, name);
    assert!(
        !output.status.success(),
        "invalid issuance must be rejected"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("certificate request"),
        "expected a request rejection, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(state(ca), before, "rejection must preserve the authority");
}

#[test]
fn external_request_issues_a_signing_only_leaf_without_creating_a_holder_key() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let (key, csr) = request(&scratch.path().join("holder"), 3072, 65537, "-sha256");
    let holder_before = state(&scratch.path().join("holder"));
    let private_before = state(&ca.join("private"));
    let body: serde_json::Value =
        serde_json::from_slice(&success(issue(&ca, &csr, "personal-document"))).unwrap();
    assert!(body.get("private_key").is_none());
    let cert = PathBuf::from(body["certificate"].as_str().unwrap());
    assert!(body["serial"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert_eq!(state(&ca.join("private")), private_before);
    assert_eq!(state(&scratch.path().join("holder")), holder_before);
    let expected = openssl(&["req", "-in", csr.to_str().unwrap(), "-pubkey", "-noout"]);
    let actual = openssl(&["x509", "-in", cert.to_str().unwrap(), "-pubkey", "-noout"]);
    assert_eq!(actual, expected, "issued public key must equal the request");
    let text = String::from_utf8(openssl(&[
        "x509",
        "-in",
        cert.to_str().unwrap(),
        "-text",
        "-noout",
    ]))
    .unwrap();
    assert!(text.contains("Public-Key: (3072 bit)"));
    assert!(text.contains("CA:FALSE"));
    assert!(text.contains("Digital Signature, Non Repudiation"));
    assert!(!text.contains("X509v3 Extended Key Usage"));
    assert!(!text.contains("X509v3 Subject Alternative Name"));
    openssl(&[
        "verify",
        "-CAfile",
        ca.join("ca.crt.pem").to_str().unwrap(),
        cert.to_str().unwrap(),
    ]);
    let public = scratch.path().join("public.pem");
    let statement = scratch.path().join("statement.bin");
    let signature = scratch.path().join("statement.sig");
    fs::write(&public, actual).unwrap();
    fs::write(&statement, b"External holder signature fixture").unwrap();
    openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        key.to_str().unwrap(),
        "-out",
        signature.to_str().unwrap(),
        statement.to_str().unwrap(),
    ]);
    openssl(&[
        "dgst",
        "-sha256",
        "-verify",
        public.to_str().unwrap(),
        "-signature",
        signature.to_str().unwrap(),
        statement.to_str().unwrap(),
    ]);
    success(pki(
        &ca,
        &["revoke", "--serial", body["serial"].as_str().unwrap()],
    ));
    success(pki(&ca, &["gen-crl"]));
    let revoked = Command::new("openssl")
        .args(["verify", "-crl_check", "-CAfile"])
        .arg(ca.join("ca.crt.pem"))
        .arg("-CRLfile")
        .arg(ca.join("crl/crl.pem"))
        .arg(cert)
        .output()
        .unwrap();
    assert!(!revoked.status.success());
    assert!(String::from_utf8_lossy(&revoked.stderr).contains("certificate revoked"));
}

#[test]
fn malformed_oversized_private_and_tampered_requests_preserve_the_authority() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let (key, csr) = request(&scratch.path().join("holder"), 3072, 65537, "-sha256");
    let valid = fs::read(&csr).unwrap();
    let mut der = openssl(&["req", "-in", csr.to_str().unwrap(), "-outform", "DER"]);
    *der.last_mut().unwrap() ^= 1;
    let encoded = STANDARD.encode(der);
    let lines = encoded
        .as_bytes()
        .chunks(64)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let tampered = format!(
        "-----BEGIN CERTIFICATE REQUEST-----\n{lines}\n-----END CERTIFICATE REQUEST-----\n"
    );
    for (index, value) in [
        Vec::new(),
        vec![b' '; REQUEST_LIMIT + 1],
        fs::read(key).unwrap(),
        [valid.as_slice(), valid.as_slice()].concat(),
        [valid.as_slice(), b"unexpected trailing material"].concat(),
        tampered.into_bytes(),
    ]
    .into_iter()
    .enumerate()
    {
        let bad = scratch.path().join(format!("invalid-{index}.pem"));
        fs::write(&bad, value).unwrap();
        rejected_without_mutation(&ca, &bad, &format!("invalid-{index}"));
    }
}

#[test]
fn unsupported_size_exponent_and_signature_digest_are_rejected_before_issuance() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    for (index, (bits, exponent, digest)) in [
        (2048, 65537, "-sha256"),
        (3072, 3, "-sha256"),
        (3072, 65537, "-sha384"),
    ]
    .into_iter()
    .enumerate()
    {
        let (_, csr) = request(
            &scratch.path().join(format!("holder-{index}")),
            bits,
            exponent,
            digest,
        );
        rejected_without_mutation(&ca, &csr, &format!("unsupported-{index}"));
    }
}

#[test]
fn a_request_cannot_overwrite_an_existing_certificate_or_escape_the_output_directory() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let (_, csr) = request(&scratch.path().join("holder"), 3072, 65537, "-sha256");
    success(issue(&ca, &csr, "existing"));
    rejected_without_mutation(&ca, &csr, "existing");
    for name in ["../outside", "/absolute", "bad/name", "", "."] {
        rejected_without_mutation(&ca, &csr, name);
    }
    assert!(!scratch.path().join("outside.crt.pem").exists());
}

#[test]
fn a_script_cannot_substitute_a_certificate_with_a_different_holder_key() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let (_, original) = request(&scratch.path().join("first-holder"), 3072, 65537, "-sha256");
    let body: serde_json::Value =
        serde_json::from_slice(&success(issue(&ca, &original, "first"))).unwrap();
    let (_, requested) = request(
        &scratch.path().join("second-holder"),
        3072,
        65537,
        "-sha256",
    );
    let custom = scratch.path().join("scripts");
    fs::create_dir(&custom).unwrap();
    let replacement = fs::read(body["certificate"].as_str().unwrap()).unwrap();
    fs::write(custom.join("replacement.pem"), replacement).unwrap();
    fs::write(
        custom.join("issue-request.sh"),
        "#!/usr/bin/env bash\nset -euo pipefail\n\
         cp \"$(dirname \"$0\")/replacement.pem\" \"$PKI_CA_DIR/certs/substituted.crt.pem\"\n",
    )
    .unwrap();
    let output = pki_with_scripts(
        &ca,
        &custom,
        &[
            "issue-request",
            "--csr",
            requested.to_str().unwrap(),
            "--name",
            "substituted",
            "--json",
        ],
    );
    assert!(
        !output.status.success(),
        "a different public key must be rejected"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("certificate request"),
        "expected a public request mismatch, got {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
