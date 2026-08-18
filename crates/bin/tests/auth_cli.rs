//! End-to-end checks of the `auth` command group through the compiled
//! binary.

use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use domain::crypto::recovery::RECOVERY_CODE_COUNT;
use domain::crypto::totp::TotpProvider;

fn binary() -> Command {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
}

/// Runs the binary with the given arguments, feeding `input` to stdin.
/// Returns (success, stdout, stderr).
fn run_with_stdin(args: &[&str], input: &str) -> (bool, String, String) {
    let mut child = binary()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    child
        .stdin
        .as_mut()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("stdin accepts the input");
    let output = child.wait_with_output().expect("the binary finishes");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn hash_password_then_verify_password_round_trip() {
    let (ok, stdout, stderr) =
        run_with_stdin(&["auth", "hash-password"], "correct horse battery staple\n");
    assert!(ok, "hash-password failed: {stderr}");
    let phc = stdout.trim().to_string();
    assert!(
        phc.starts_with("$argon2id$v=19$m=262144,t=2,p=1$"),
        "got: {phc}"
    );

    let (ok, stdout, stderr) = run_with_stdin(
        &["auth", "verify-password", "--hash", &phc],
        "correct horse battery staple\n",
    );
    assert!(ok, "verify-password failed: {stderr}");
    assert_eq!(stdout.trim(), "match");

    let (ok, stdout, _) = run_with_stdin(
        &["auth", "verify-password", "--hash", &phc],
        "wrong password\n",
    );
    assert!(!ok, "a mismatch must exit nonzero");
    assert_eq!(stdout.trim(), "mismatch");
}

/// Returns a six-digit code that is not valid for `secret` in the
/// current, previous, or next time step, so a verification of it must
/// be rejected regardless of clock skew tolerance.
fn code_not_currently_valid(secret: &[u8], now: u64) -> String {
    let provider = infrastructure::TotpRsProvider::new();
    let valid: Vec<String> = [now.saturating_sub(30), now, now + 30]
        .iter()
        .map(|t| provider.current_code(secret, *t).expect("a code derives"))
        .collect();
    (0..1_000_000u32)
        .map(|n| format!("{n:06}"))
        .find(|candidate| !valid.contains(candidate))
        .expect("at most three of the million six-digit codes are valid")
}

#[test]
fn verify_password_exit_code_distinguishes_match_from_mismatch() {
    let (ok, stdout, stderr) = run_with_stdin(&["auth", "hash-password"], "open sesame\n");
    assert!(ok, "hash-password failed: {stderr}");
    let phc = stdout.trim().to_string();

    // The right password exits zero, in human and JSON modes alike.
    let (ok, _, stderr) = run_with_stdin(
        &["auth", "verify-password", "--hash", &phc],
        "open sesame\n",
    );
    assert!(ok, "a matching password must exit zero: {stderr}");
    let (ok, _, stderr) = run_with_stdin(
        &["--json", "auth", "verify-password", "--hash", &phc],
        "open sesame\n",
    );
    assert!(
        ok,
        "a matching password must exit zero with --json: {stderr}"
    );

    // The wrong password still reports the mismatch but exits nonzero.
    let (ok, stdout, _) = run_with_stdin(
        &["auth", "verify-password", "--hash", &phc],
        "wrong password\n",
    );
    assert!(!ok, "a mismatch must exit nonzero");
    assert_eq!(stdout.trim(), "mismatch");
    let (ok, _, _) = run_with_stdin(
        &["--json", "auth", "verify-password", "--hash", &phc],
        "wrong password\n",
    );
    assert!(!ok, "a mismatch must exit nonzero with --json");
}

#[test]
fn totp_verify_exit_code_distinguishes_accepted_from_rejected() {
    let dir = std::env::temp_dir().join(format!("despacho-auth-exit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir is writable");
    let secret_path = dir.join("totp-secret.b32");

    let output = binary()
        .args(["auth", "totp", "enroll", "--user", "user@example.com"])
        .arg("--secret-out")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(output.status.success(), "enroll must exit zero");

    let stored = std::fs::read_to_string(&secret_path).expect("the secret file was written");
    let secret = infrastructure::decode_base32_secret(&stored).expect("the file holds base32");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is past the epoch")
        .as_secs();
    let wrong_code = code_not_currently_valid(&secret, now);

    // A wrong code still reports the rejection but exits nonzero.
    let output = binary()
        .args(["auth", "totp", "verify", "--code", &wrong_code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        !output.status.success(),
        "a rejected code must exit nonzero"
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "rejected");
    let output = binary()
        .args(["--json", "auth", "totp", "verify", "--code", &wrong_code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        !output.status.success(),
        "a rejected code must exit nonzero with --json"
    );

    // The current code exits zero, in human and JSON modes alike.
    let good_code = infrastructure::TotpRsProvider::new()
        .current_code(&secret, now)
        .expect("a code derives from the stored secret");
    let output = binary()
        .args(["auth", "totp", "verify", "--code", &good_code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(output.status.success(), "an accepted code must exit zero");
    let output = binary()
        .args(["--json", "auth", "totp", "verify", "--code", &good_code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "an accepted code must exit zero with --json"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn hash_password_and_verify_password_emit_json_objects() {
    let (ok, stdout, stderr) =
        run_with_stdin(&["--json", "auth", "hash-password"], "json mode password\n");
    assert!(ok, "hash-password failed: {stderr}");
    let body: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("hash-password --json emits one JSON object");
    let phc = body["phc"]
        .as_str()
        .expect("the object carries a phc string")
        .to_string();
    assert!(phc.starts_with("$argon2id$"), "got: {phc}");

    let (ok, stdout, stderr) = run_with_stdin(
        &["--json", "auth", "verify-password", "--hash", &phc],
        "json mode password\n",
    );
    assert!(ok, "a matching password must exit zero: {stderr}");
    let body: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("verify-password --json emits one JSON object");
    assert_eq!(body["match"], serde_json::Value::Bool(true));

    let (ok, stdout, _) = run_with_stdin(
        &["--json", "auth", "verify-password", "--hash", &phc],
        "wrong password\n",
    );
    assert!(!ok, "a mismatch must exit nonzero with --json");
    let body: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("verify-password --json emits one JSON object");
    assert_eq!(body["match"], serde_json::Value::Bool(false));
}

#[test]
fn totp_enroll_and_verify_emit_json_objects() {
    let dir = std::env::temp_dir().join(format!("despacho-auth-json-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir is writable");
    let secret_path = dir.join("totp-secret.b32");

    let output = binary()
        .args([
            "--json",
            "auth",
            "totp",
            "enroll",
            "--user",
            "user@example.com",
        ])
        .arg("--secret-out")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "enroll failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("enroll --json emits one JSON object");
    assert!(
        body["otpauth_uri"]
            .as_str()
            .expect("the object carries the otpauth uri")
            .starts_with("otpauth://totp/despacho:user%40example.com?"),
        "got: {body}"
    );
    assert!(
        body["secret_base32"].is_string(),
        "the object carries the base32 secret"
    );
    assert_eq!(
        body["recovery_codes"]
            .as_array()
            .expect("the object carries the recovery codes")
            .len(),
        RECOVERY_CODE_COUNT
    );
    assert_eq!(
        body["secret_path"]
            .as_str()
            .expect("secret_path is a string"),
        secret_path.to_str().unwrap()
    );

    let stored = std::fs::read_to_string(&secret_path).expect("the secret file was written");
    let secret = infrastructure::decode_base32_secret(&stored).expect("the file holds base32");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is past the epoch")
        .as_secs();

    let good_code = infrastructure::TotpRsProvider::new()
        .current_code(&secret, now)
        .expect("a code derives from the stored secret");
    let output = binary()
        .args(["--json", "auth", "totp", "verify", "--code", &good_code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(output.status.success(), "an accepted code must exit zero");
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("verify --json emits one JSON object");
    assert_eq!(body["accepted"], serde_json::Value::Bool(true));

    let wrong_code = code_not_currently_valid(&secret, now);
    let output = binary()
        .args(["--json", "auth", "totp", "verify", "--code", &wrong_code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        !output.status.success(),
        "a rejected code must exit nonzero with --json"
    );
    let body: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("verify --json emits one JSON object");
    assert_eq!(body["accepted"], serde_json::Value::Bool(false));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn totp_enroll_writes_the_secret_and_a_current_code_verifies() {
    let dir = std::env::temp_dir().join(format!("despacho-auth-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir is writable");
    let secret_path = dir.join("totp-secret.b32");

    let output = binary()
        .args(["auth", "totp", "enroll", "--user", "user@example.com"])
        .arg("--secret-out")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "enroll failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("otpauth://totp/despacho:user%40example.com?"),
        "got: {stdout}"
    );
    // Recovery codes print as indented "XXXXX-XXXXX" lines.
    let recovery_lines = stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.len() == 11 && line.as_bytes()[5] == b'-')
        .count();
    assert_eq!(recovery_lines, RECOVERY_CODE_COUNT);

    let stored = std::fs::read_to_string(&secret_path).expect("the secret file was written");
    let secret = infrastructure::decode_base32_secret(&stored).expect("the file holds base32");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock is past the epoch")
        .as_secs();
    let code = infrastructure::TotpRsProvider::new()
        .current_code(&secret, now)
        .expect("a code derives from the stored secret");

    let output = binary()
        .args(["auth", "totp", "verify", "--code", &code])
        .arg("--secret-file")
        .arg(&secret_path)
        .output()
        .expect("the binary runs");
    assert!(
        output.status.success(),
        "verify failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "accepted");

    std::fs::remove_dir_all(&dir).ok();
}
