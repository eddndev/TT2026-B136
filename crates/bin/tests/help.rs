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
}
