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
        phc.starts_with("$argon2id$v=19$m=47104,t=1,p=1$"),
        "got: {phc}"
    );

    let (ok, stdout, stderr) = run_with_stdin(
        &["auth", "verify-password", "--hash", &phc],
        "correct horse battery staple\n",
    );
    assert!(ok, "verify-password failed: {stderr}");
    assert_eq!(stdout.trim(), "match");

    let (ok, stdout, stderr) = run_with_stdin(
        &["auth", "verify-password", "--hash", &phc],
        "wrong password\n",
    );
    assert!(ok, "verify-password failed: {stderr}");
    assert_eq!(stdout.trim(), "mismatch");
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
