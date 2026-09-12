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
