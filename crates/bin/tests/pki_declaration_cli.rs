//! Real internal declaration certificates, issued without authentication EKUs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn pki(ca: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .arg("pki")
        .arg("--scripts-dir")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki"))
        .args(args)
        .env("PKI_CA_DIR", ca)
        .output()
        .unwrap()
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

fn issue(ca: &Path, name: &str, purpose: Option<&str>) -> (PathBuf, PathBuf) {
    let mut args = vec!["issue", "--cn", name, "--json"];
    if let Some(purpose) = purpose {
        args.extend(["--purpose", purpose]);
    }
    let output = success(pki(ca, &args));
    let body: serde_json::Value = serde_json::from_slice(&output).unwrap();
    (
        PathBuf::from(body["certificate"].as_str().unwrap()),
        PathBuf::from(body["private_key"].as_str().unwrap()),
    )
}

#[test]
fn declaration_leaf_signs_with_its_own_key_and_can_be_revoked() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let (partner, _) = issue(&ca, "Existing Partner", None);
    let partner_before = fs::read(&partner).unwrap();
    let (cert, key) = issue(&ca, "Synthetic Judge", Some("participant-declaration"));
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
    assert_eq!(fs::read(&partner).unwrap(), partner_before);
    let previous = String::from_utf8(openssl(&[
        "x509",
        "-in",
        partner.to_str().unwrap(),
        "-text",
        "-noout",
    ]))
    .unwrap();
    assert!(previous.contains("TLS Web Client Authentication, E-mail Protection"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(fs::metadata(&key).unwrap().permissions().mode() & 0o077, 0);
    }
    let declaration = scratch.path().join("declaration.bin");
    let signature = scratch.path().join("declaration.sig");
    let public_key = scratch.path().join("public.pem");
    fs::write(
        &declaration,
        b"Synthetic internal declaration, no legal identity claim",
    )
    .unwrap();
    fs::write(
        &public_key,
        openssl(&["x509", "-in", cert.to_str().unwrap(), "-pubkey", "-noout"]),
    )
    .unwrap();
    openssl(&[
        "dgst",
        "-sha256",
        "-sign",
        key.to_str().unwrap(),
        "-out",
        signature.to_str().unwrap(),
        declaration.to_str().unwrap(),
    ]);
    assert_eq!(fs::metadata(&signature).unwrap().len(), 384);
    openssl(&[
        "dgst",
        "-sha256",
        "-verify",
        public_key.to_str().unwrap(),
        "-signature",
        signature.to_str().unwrap(),
        declaration.to_str().unwrap(),
    ]);
    let serial = String::from_utf8(openssl(&[
        "x509",
        "-in",
        cert.to_str().unwrap(),
        "-serial",
        "-noout",
    ]))
    .unwrap();
    success(pki(
        &ca,
        &[
            "revoke",
            "--serial",
            serial.trim().strip_prefix("serial=").unwrap(),
        ],
    ));
    success(pki(&ca, &["gen-crl"]));
    let rejected = Command::new("openssl")
        .args(["verify", "-crl_check", "-CAfile"])
        .arg(ca.join("ca.crt.pem"))
        .arg("-CRLfile")
        .arg(ca.join("crl/crl.pem"))
        .arg(&cert)
        .output()
        .unwrap();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stderr).contains("certificate revoked"));
}

#[test]
fn declaration_issuance_cannot_overwrite_an_existing_partner_key() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let (cert, key) = issue(&ca, "Existing Identity", None);
    let before = (fs::read(&cert).unwrap(), fs::read(&key).unwrap());
    let output = pki(
        &ca,
        &[
            "issue",
            "--cn",
            "Existing Identity",
            "--purpose",
            "participant-declaration",
        ],
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("certificate already exists"));
    assert_eq!((fs::read(&cert).unwrap(), fs::read(&key).unwrap()), before);
}

#[test]
fn unsupported_purpose_is_rejected_before_creating_runtime_files() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    let output = pki(
        &ca,
        &["issue", "--cn", "Synthetic", "--purpose", "official-firel"],
    );
    assert!(!output.status.success());
    assert!(!ca.exists());
}

#[test]
fn declaration_issuance_rejects_an_incompatible_usage_from_custom_scripts() {
    let scratch = tempfile::tempdir().unwrap();
    let ca = scratch.path().join("ca");
    success(pki(&ca, &["init-ca"]));
    let scripts = scratch.path().join("scripts");
    fs::create_dir(&scripts).unwrap();
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pki");
    for name in ["issue-cert.sh", "issue-declaration-cert.sh"] {
        fs::copy(original.join(name), scripts.join(name)).unwrap();
    }
    let config = fs::read_to_string(original.join("openssl.cnf")).unwrap();
    let config = config.replace(
        "[ v3_internal_declaration ]",
        "[ v3_internal_declaration ]\nextendedKeyUsage = clientAuth",
    );
    fs::write(scripts.join("openssl.cnf"), config).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .arg("pki")
        .arg("--scripts-dir")
        .arg(&scripts)
        .args([
            "issue",
            "--cn",
            "Incompatible Usage",
            "--purpose",
            "participant-declaration",
        ])
        .env("PKI_CA_DIR", &ca)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "an incompatible leaf must not be reported as issued"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("certificate issuance failed"));
}
