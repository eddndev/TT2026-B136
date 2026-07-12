//! End-to-end checks of `vault encrypt`, `vault decrypt`, and
//! `vault rotate-kek` through the compiled binary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use base64::Engine;

const DOC_ID: &str = "00112233-4455-6677-8899-aabbccddeeff";
const KEK: [u8; 32] = [0x42; 32];
const NEW_KEK: [u8; 32] = [0x43; 32];

/// A per-test scratch directory, removed when the value drops.
struct ScratchDir(PathBuf);

impl ScratchDir {
    fn new(tag: &str) -> Self {
        let mut dir = std::env::temp_dir();
        dir.push(format!("despacho-vault-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("scratch dir is creatable");
        Self(dir)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn base64_of(key: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(key)
}

fn run_cli(args: &[&str], envs: &[(&str, String)]) -> Output {
    let exe = env!("CARGO_BIN_EXE_despacho-cli");
    let mut cmd = Command::new(exe);
    cmd.args(args);
    for (name, value) in envs {
        cmd.env(name, value);
    }
    cmd.output().expect("the binary runs")
}

fn assert_success(output: &Output, when: &str) {
    assert!(
        output.status.success(),
        "{when} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sample_plaintext() -> Vec<u8> {
    (0..1000u32).map(|i| (i % 251) as u8).collect()
}

fn encrypt_sample(dir: &ScratchDir, kek_b64: &str) -> (PathBuf, PathBuf) {
    let plain_path = dir.path("document.bin");
    fs::write(&plain_path, sample_plaintext()).expect("plaintext is writable");
    let output = run_cli(
        &[
            "vault",
            "encrypt",
            plain_path.to_str().unwrap(),
            "--doc-id",
            DOC_ID,
            "--version",
            "3",
        ],
        &[("KEK_BASE64", kek_b64.to_string())],
    );
    assert_success(&output, "vault encrypt");
    let enc_path = dir.path("document.bin.enc");
    assert!(enc_path.is_file(), "encrypt writes <file>.enc");
    (plain_path, enc_path)
}

fn decrypt_args<'a>(enc_path: &'a Path, version: &'a str) -> Vec<&'a str> {
    vec![
        "vault",
        "decrypt",
        enc_path.to_str().unwrap(),
        "--doc-id",
        DOC_ID,
        "--version",
        version,
    ]
}

#[test]
fn encrypt_then_decrypt_round_trips_byte_identical() {
    let dir = ScratchDir::new("roundtrip");
    let kek = base64_of(&KEK);
    let (_, enc_path) = encrypt_sample(&dir, &kek);

    // Decrypt to a file.
    let out_path = dir.path("restored.bin");
    let mut args = decrypt_args(&enc_path, "3");
    args.push("--out");
    args.push(out_path.to_str().unwrap());
    let output = run_cli(&args, &[("KEK_BASE64", kek.clone())]);
    assert_success(&output, "vault decrypt --out");
    assert_eq!(fs::read(&out_path).unwrap(), sample_plaintext());

    // Decrypt to standard output.
    let output = run_cli(&decrypt_args(&enc_path, "3"), &[("KEK_BASE64", kek)]);
    assert_success(&output, "vault decrypt to stdout");
    assert_eq!(output.stdout, sample_plaintext());
}

#[test]
fn decrypt_with_the_wrong_version_fails() {
    let dir = ScratchDir::new("wrong-version");
    let kek = base64_of(&KEK);
    let (_, enc_path) = encrypt_sample(&dir, &kek);

    let output = run_cli(&decrypt_args(&enc_path, "4"), &[("KEK_BASE64", kek)]);
    assert!(!output.status.success(), "a wrong version must not decrypt");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("authenticated decryption failed"),
        "stderr should name the authentication failure, got: {stderr}"
    );
}

#[test]
fn decrypting_a_corrupted_package_fails_with_an_authentication_error() {
    let dir = ScratchDir::new("corrupted");
    let kek = base64_of(&KEK);
    let (_, enc_path) = encrypt_sample(&dir, &kek);

    // Flip one byte inside the sealed document region (the file tail is the
    // authentication tag, so any change there must be detected).
    let mut bytes = fs::read(&enc_path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    fs::write(&enc_path, &bytes).unwrap();

    let output = run_cli(&decrypt_args(&enc_path, "3"), &[("KEK_BASE64", kek)]);
    assert!(
        !output.status.success(),
        "a corrupted package must not decrypt"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("authenticated decryption failed"),
        "stderr should name the authentication failure, got: {stderr}"
    );
}

#[test]
fn vault_commands_emit_json_objects() {
    let dir = ScratchDir::new("json");
    let kek = base64_of(&KEK);
    let new_kek = base64_of(&NEW_KEK);
    let plain_path = dir.path("document.bin");
    fs::write(&plain_path, sample_plaintext()).expect("plaintext is writable");

    let output = run_cli(
        &[
            "--json",
            "vault",
            "encrypt",
            plain_path.to_str().unwrap(),
            "--doc-id",
            DOC_ID,
            "--version",
            "3",
        ],
        &[("KEK_BASE64", kek.clone())],
    );
    assert_success(&output, "vault encrypt --json");
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("encrypt --json emits one JSON object");
    let enc_path = dir.path("document.bin.enc");
    assert_eq!(
        body["package_path"]
            .as_str()
            .expect("package_path is a string"),
        enc_path.to_str().unwrap()
    );

    // Decrypt to a file names the file; decrypt to standard output stays
    // raw plaintext so pipes keep working.
    let out_path = dir.path("restored.bin");
    let mut args = decrypt_args(&enc_path, "3");
    args.insert(0, "--json");
    args.push("--out");
    args.push(out_path.to_str().unwrap());
    let output = run_cli(&args, &[("KEK_BASE64", kek.clone())]);
    assert_success(&output, "vault decrypt --json --out");
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("decrypt --json emits one JSON object");
    assert_eq!(
        body["out"].as_str().expect("out is a string"),
        out_path.to_str().unwrap()
    );
    assert_eq!(fs::read(&out_path).unwrap(), sample_plaintext());

    let mut args = decrypt_args(&enc_path, "3");
    args.insert(0, "--json");
    let output = run_cli(&args, &[("KEK_BASE64", kek.clone())]);
    assert_success(&output, "vault decrypt --json to stdout");
    assert_eq!(output.stdout, sample_plaintext());

    let output = run_cli(
        &[
            "--json",
            "vault",
            "rotate-kek",
            "--file",
            enc_path.to_str().unwrap(),
        ],
        &[("KEK_BASE64", kek), ("NEW_KEK_BASE64", new_kek)],
    );
    assert_success(&output, "vault rotate-kek --json");
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("rotate-kek --json emits one JSON object");
    assert_eq!(
        body["rewritten"].as_str().expect("rewritten is a string"),
        enc_path.to_str().unwrap()
    );
}

#[test]
fn rotating_the_kek_keeps_the_package_decryptable_only_under_the_new_kek() {
    let dir = ScratchDir::new("rotation");
    let old_kek = base64_of(&KEK);
    let new_kek = base64_of(&NEW_KEK);
    let (_, enc_path) = encrypt_sample(&dir, &old_kek);

    let output = run_cli(
        &["vault", "rotate-kek", "--file", enc_path.to_str().unwrap()],
        &[
            ("KEK_BASE64", old_kek.clone()),
            ("NEW_KEK_BASE64", new_kek.clone()),
        ],
    );
    assert_success(&output, "vault rotate-kek");

    // The new key decrypts the same plaintext.
    let output = run_cli(&decrypt_args(&enc_path, "3"), &[("KEK_BASE64", new_kek)]);
    assert_success(&output, "vault decrypt after rotation");
    assert_eq!(output.stdout, sample_plaintext());

    // The old key no longer works.
    let output = run_cli(&decrypt_args(&enc_path, "3"), &[("KEK_BASE64", old_kek)]);
    assert!(
        !output.status.success(),
        "the old kek must stop working after rotation"
    );
}
