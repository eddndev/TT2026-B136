//! Fitness rule: secret material never reaches the diagnostic stream.
//!
//! The binary writes command results to standard output and diagnostics to
//! standard error. Output files and standard output can be quarantined by a
//! deployment (file permissions, pipes), but log streams are routinely
//! shipped to collectors the operator does not control, so a secret leaking
//! into the logs is a release blocker, not a cosmetic bug.
//!
//! Each test runs the compiled binary at the most verbose log level and
//! asserts that a known secret input never appears on standard error, in
//! any encoding a careless logging statement would plausibly use.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use base64::Engine;

const DOC_ID: &str = "00112233-4455-6677-8899-aabbccddeeff";

/// Key encryption key for the vault flow: 32 distinct bytes, so its full
/// encodings cannot collide with ordinary log text by accident.
fn kek_bytes() -> Vec<u8> {
    (0u8..32).collect()
}

fn base64_of(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, byte| {
        let _ = write!(acc, "{byte:02x}");
        acc
    })
}

/// Runs the binary with trace-level logging and returns its output.
/// Panics on a non-zero exit, so every assertion below inspects the
/// standard error of a complete, successful run.
fn run_traced(args: &[&str], envs: &[(&str, &str)], stdin: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_despacho-cli"));
    command
        .args(args)
        .env("RUST_LOG", "trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in envs {
        command.env(name, value);
    }
    let mut child = command.spawn().expect("the binary runs");
    if let Some(input) = stdin {
        child
            .stdin
            .as_mut()
            .expect("stdin is piped")
            .write_all(input.as_bytes())
            .expect("stdin accepts the input");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("the binary finishes");
    assert!(
        output.status.success(),
        "`{}` failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Decodes the diagnostic stream and checks it carried something: at trace
/// level the binary always logs at least the received command, so an empty
/// stream means the capture is not observing the logs and the leak
/// assertions would pass vacuously.
fn diagnostics_of(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(
        !stderr.is_empty(),
        "trace-level logging produced no diagnostics; the capture is broken"
    );
    stderr
}

#[test]
fn vault_encrypt_never_logs_the_kek() {
    let dir = tempfile::tempdir().expect("a scratch directory is creatable");
    let plain_path = dir.path().join("document.bin");
    std::fs::write(&plain_path, b"confidential document body").expect("plaintext is writable");

    let kek = kek_bytes();
    let kek_b64 = base64_of(&kek);
    let output = run_traced(
        &[
            "vault",
            "encrypt",
            plain_path.to_str().unwrap(),
            "--doc-id",
            DOC_ID,
        ],
        &[("KEK_BASE64", kek_b64.as_str())],
        None,
    );
    let stderr = diagnostics_of(&output);

    // The key as handed to the process, its hex expansions, and the Debug
    // rendering of the raw byte vector (the most likely accidental leak in
    // Rust logging) must all be absent.
    let hex = hex_lower(&kek);
    for (encoding, label) in [
        (kek_b64.clone(), "base64"),
        (hex.clone(), "lowercase hex"),
        (hex.to_uppercase(), "uppercase hex"),
        (format!("{kek:?}"), "debug byte list"),
    ] {
        assert!(
            !stderr.contains(&encoding),
            "the kek reached the log stream as {label}: {encoding}"
        );
    }
}

#[test]
fn hash_password_never_logs_the_password() {
    const PASSWORD: &str = "redaction-probe-password-3f9c";

    let output = run_traced(
        &["auth", "hash-password"],
        &[],
        Some(&format!("{PASSWORD}\n")),
    );
    let stderr = diagnostics_of(&output);
    assert!(
        !stderr.contains(PASSWORD),
        "the password reached the log stream: {stderr}"
    );
}

#[test]
fn totp_enroll_never_logs_the_base32_secret() {
    let dir = tempfile::tempdir().expect("a scratch directory is creatable");
    let secret_path = dir.path().join("totp-secret.b32");

    let output = run_traced(
        &[
            "auth",
            "totp",
            "enroll",
            "--user",
            "user@example.com",
            "--secret-out",
            secret_path.to_str().unwrap(),
        ],
        &[],
        None,
    );
    let stderr = diagnostics_of(&output);

    // The enrollment secret is printed on standard output on purpose; the
    // same string must not also travel on the diagnostic stream. A leak of
    // the otpauth URI would be caught too, because the URI embeds the
    // secret.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let secret = stdout
        .lines()
        .find_map(|line| line.strip_prefix("secret-base32: "))
        .expect("enroll prints the base32 secret on standard output")
        .trim()
        .to_string();
    assert!(!secret.is_empty(), "the printed secret must not be empty");
    assert!(
        !stderr.contains(&secret),
        "the base32 secret reached the log stream"
    );
}
