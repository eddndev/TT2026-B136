//! The binary starts and prints usage for `--help`.

use std::process::Command;

#[test]
fn help_succeeds_and_names_the_binary() {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let output = Command::new(exe)
        .arg("--help")
        .output()
        .expect("the binary runs");

    assert!(output.status.success(), "--help must exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("despacho-cli"),
        "usage text should name the binary"
    );
    assert!(
        stdout.contains("serve"),
        "usage text should expose the local http application"
    );
}

#[test]
fn serve_exposes_nonzero_resource_limits() {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let help = Command::new(exe)
        .args(["serve", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8(help.stdout).unwrap();
    for flag in ["--max-in-flight-requests", "--max-blocking-operations"] {
        assert!(help.contains(flag), "missing limit {flag}");
        let rejected = Command::new(exe)
            .args([
                "serve",
                flag,
                "0",
                "--signer-cert",
                "unused",
                "--signer-key",
                "unused",
            ])
            .output()
            .unwrap();
        assert!(!rejected.status.success());
        assert!(String::from_utf8_lossy(&rejected.stderr).contains("zero"));
    }
}

#[test]
fn serve_requires_an_explicit_native_document_library() {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let output = Command::new(exe)
        .env_remove("DOCUMENT_QPDF_LIBRARY")
        .args(["serve", "--signer-cert", "unused", "--signer-key", "unused"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("--qpdf-library"));
    assert!(!error.contains("cannot read signer"));
}

#[test]
fn serve_exposes_bounded_serial_deadline_processing() {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let output = Command::new(exe)
        .args(["serve", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for flag in ["--deadline-page-limit", "--deadline-poll-ms"] {
        assert!(help.contains(flag), "missing deadline configuration {flag}");
    }
    for (flag, value) in [
        ("--deadline-page-limit", "0"),
        ("--deadline-page-limit", "101"),
        ("--deadline-poll-ms", "0"),
        ("--deadline-poll-ms", "4294967296"),
    ] {
        let rejected = Command::new(exe)
            .env_remove("DOCUMENT_QPDF_LIBRARY")
            .args([
                "serve",
                flag,
                value,
                "--qpdf-library",
                "unused",
                "--signer-cert",
                "unused",
                "--signer-key",
                "unused",
            ])
            .output()
            .unwrap();
        assert_eq!(rejected.status.code(), Some(2));
        let stderr = String::from_utf8(rejected.stderr).unwrap();
        assert!(stderr.contains(flag));
        assert!(!stderr.contains("cannot locate native"));
    }
}
