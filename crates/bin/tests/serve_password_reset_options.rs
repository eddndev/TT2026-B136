//! Server recovery settings are explicit and validated before adapter startup.

use std::{
    ops::{Deref, DerefMut},
    process::{Command, Output},
};

const PRIVATE_KEY: &str = "private-password-reset-cli-fixture";
const OPTIONS: &[(&str, &str, &str)] = &[
    (
        "password-reset-email-from",
        "TT_PASSWORD_RESET_EMAIL_FROM",
        "recovery@example.test",
    ),
    (
        "password-reset-public-url",
        "TT_PASSWORD_RESET_PUBLIC_URL",
        "https://qadra.example.test/",
    ),
    (
        "password-reset-ttl-seconds",
        "TT_PASSWORD_RESET_TTL_SECONDS",
        "713",
    ),
    (
        "password-reset-max-pending",
        "TT_PASSWORD_RESET_MAX_PENDING",
        "2",
    ),
    (
        "password-reset-request-global-max",
        "TT_PASSWORD_RESET_REQUEST_GLOBAL_MAX",
        "13",
    ),
    (
        "password-reset-request-global-window-seconds",
        "TT_PASSWORD_RESET_REQUEST_GLOBAL_WINDOW_SECONDS",
        "61",
    ),
    (
        "password-reset-request-email-max",
        "TT_PASSWORD_RESET_REQUEST_EMAIL_MAX",
        "3",
    ),
    (
        "password-reset-request-email-window-seconds",
        "TT_PASSWORD_RESET_REQUEST_EMAIL_WINDOW_SECONDS",
        "67",
    ),
    (
        "password-reset-complete-global-max",
        "TT_PASSWORD_RESET_COMPLETE_GLOBAL_MAX",
        "17",
    ),
    (
        "password-reset-complete-global-window-seconds",
        "TT_PASSWORD_RESET_COMPLETE_GLOBAL_WINDOW_SECONDS",
        "71",
    ),
    (
        "password-reset-complete-token-max",
        "TT_PASSWORD_RESET_COMPLETE_TOKEN_MAX",
        "5",
    ),
    (
        "password-reset-complete-token-window-seconds",
        "TT_PASSWORD_RESET_COMPLETE_TOKEN_WINDOW_SECONDS",
        "73",
    ),
    (
        "password-reset-redis-connect-ms",
        "TT_PASSWORD_RESET_REDIS_CONNECT_MS",
        "83",
    ),
    (
        "password-reset-redis-io-ms",
        "TT_PASSWORD_RESET_REDIS_IO_MS",
        "89",
    ),
    (
        "password-reset-email-connect-ms",
        "TT_PASSWORD_RESET_EMAIL_CONNECT_MS",
        "97",
    ),
    (
        "password-reset-email-total-ms",
        "TT_PASSWORD_RESET_EMAIL_TOTAL_MS",
        "101",
    ),
];

struct Invocation {
    command: Command,
    _directory: tempfile::TempDir,
}
impl Deref for Invocation {
    type Target = Command;
    fn deref(&self) -> &Self::Target {
        &self.command
    }
}
impl DerefMut for Invocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.command
    }
}

fn command() -> Invocation {
    let directory = tempfile::tempdir().unwrap();
    // Stop dotenv discovery here rather than inheriting a developer's parent file.
    std::fs::write(directory.path().join(".env"), b"").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_despacho-cli"));
    command.current_dir(directory.path()).env_clear().args([
        "serve",
        "--qpdf-library",
        "/nonexistent-password-reset-fixture/qpdf",
        "--signer-cert",
        "unused",
        "--signer-key",
        "unused",
    ]);
    Invocation {
        command,
        _directory: directory,
    }
}

fn configured() -> Invocation {
    let mut command = command();
    command
        .env("RESEND_API_KEY", PRIVATE_KEY)
        .env("TT_PASSWORD_RESET_ENABLED", "true");
    for (_, name, value) in OPTIONS {
        command.env(name, value);
    }
    command
}

fn diagnostic(output: Output) -> String {
    assert!(!output.status.success());
    let text = String::from_utf8(output.stderr).expect("ASCII server diagnostics");
    assert!(!text.contains(PRIVATE_KEY));
    assert!(!String::from_utf8_lossy(&output.stdout).contains(PRIVATE_KEY));
    text
}

fn reached_native_check(output: Output) {
    let text = diagnostic(output);
    assert!(
        text.contains("cannot locate native qpdf library"),
        "unexpected startup boundary: {text}"
    );
    assert!(!text.contains("password recovery settings"));
}

#[test]
fn help_lists_explicit_reset_settings_but_no_secret_flag_or_key_value() {
    let output = command()
        .env("RESEND_API_KEY", PRIVATE_KEY)
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--password-reset-enabled"));
    assert!(help.contains("TT_PASSWORD_RESET_ENABLED"));
    for (flag, name, _) in OPTIONS {
        assert!(help.contains(&format!("--{flag}")), "missing {flag}");
        assert!(help.contains(name), "missing environment option {name}");
    }
    assert!(!help.contains("--resend-api-key"));
    assert!(!help.contains("--password-reset-api-key"));
    assert!(!help.contains(PRIVATE_KEY));
}

#[test]
fn default_and_shared_key_alone_keep_recovery_and_alert_email_disabled() {
    reached_native_check(command().output().unwrap());
    reached_native_check(
        command()
            .env("RESEND_API_KEY", PRIVATE_KEY)
            .output()
            .unwrap(),
    );
}

#[test]
fn explicit_disable_keeps_saved_sender_and_origin_inert() {
    reached_native_check(
        command()
            .env("TT_PASSWORD_RESET_ENABLED", "false")
            .env("TT_PASSWORD_RESET_EMAIL_FROM", "not-an-address")
            .env(
                "TT_PASSWORD_RESET_PUBLIC_URL",
                "http://unapproved.invalid/private",
            )
            .output()
            .unwrap(),
    );
}

#[test]
fn enabling_requires_every_setting_before_native_validation_or_bind() {
    for (_, missing, _) in OPTIONS {
        let output = configured().env_remove(missing).output().unwrap();
        let text = diagnostic(output);
        assert!(
            text.contains("password recovery"),
            "missing {missing}: {text}"
        );
        assert!(!text.contains("cannot locate native"));
        assert!(!text.contains("cannot bind"));
    }
    let text = diagnostic(configured().env_remove("RESEND_API_KEY").output().unwrap());
    assert!(text.contains("password recovery"));
    assert!(!text.contains("cannot locate native"));
}

#[test]
fn complete_environment_and_command_line_configuration_reach_the_same_startup_boundary() {
    reached_native_check(configured().output().unwrap());
    let mut command = command();
    command
        .env("RESEND_API_KEY", PRIVATE_KEY)
        .arg("--password-reset-enabled");
    for (flag, _, value) in OPTIONS {
        command.arg(format!("--{flag}")).arg(value);
    }
    reached_native_check(command.output().unwrap());
}

#[test]
fn invalid_reset_and_partial_alert_settings_fail_before_opening_adapters() {
    for (name, value) in [
        ("TT_PASSWORD_RESET_PUBLIC_URL", "http://qadra.example.test/"),
        (
            "TT_PASSWORD_RESET_PUBLIC_URL",
            "https://qadra.example.test/?private=value",
        ),
        ("TT_PASSWORD_RESET_TTL_SECONDS", "0"),
        ("TT_PASSWORD_RESET_REQUEST_EMAIL_WINDOW_SECONDS", "86401"),
        ("TT_PASSWORD_RESET_EMAIL_TOTAL_MS", "10001"),
    ] {
        let text = diagnostic(configured().env(name, value).output().unwrap());
        assert!(!text.contains("cannot locate native"));
        assert!(!text.contains("cannot bind"));
    }
    let text = diagnostic(
        configured()
            .env("ALERT_EMAIL_FROM", "alerts@example.test")
            .output()
            .unwrap(),
    );
    assert!(text.contains("alert email"));
    assert!(!text.contains("cannot locate native"));
}
