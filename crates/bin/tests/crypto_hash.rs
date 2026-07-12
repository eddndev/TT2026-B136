//! End-to-end tests of the `crypto hash` subcommand against the compiled
//! binary.

use std::path::PathBuf;
use std::process::{Command, Output};

/// SHA-256 of the three bytes "abc", from the FIPS 180-4 example vectors.
const ABC_DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

fn run_binary(args: &[&str], file: &PathBuf) -> Output {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .args(args)
        .arg(file)
        .output()
        .expect("the compiled binary must run")
}

fn write_sample(dir: &tempfile::TempDir) -> PathBuf {
    let path = dir.path().join("doc.txt");
    std::fs::write(&path, b"abc").unwrap();
    path
}

#[test]
fn prints_the_lowercase_hex_digest() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let output = run_binary(&["crypto", "hash"], &path);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.trim(), ABC_DIGEST);
}

#[test]
fn json_flag_prints_a_machine_readable_object() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_sample(&dir);

    let output = run_binary(&["--json", "crypto", "hash"], &path);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let body: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(body["algorithm"], "sha-256");
    assert_eq!(body["digest"], ABC_DIGEST);
    assert_eq!(body["file"], path.display().to_string());
}

#[test]
fn fails_when_the_file_does_not_exist() {
    let missing = PathBuf::from("no-such-file.bin");

    let output = run_binary(&["crypto", "hash"], &missing);

    assert!(!output.status.success());
}
