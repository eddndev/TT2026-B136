//! End-to-end tests of the `audit` command group against the compiled
//! binary: appending entries, verifying the chain, and detecting a
//! corrupted log file.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run_binary(log_path: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .args(args)
        .env("AUDIT_LOG_PATH", log_path)
        .env("USER", "tester")
        .output()
        .expect("the compiled binary must run")
}

fn log_path(dir: &tempfile::TempDir) -> PathBuf {
    dir.path().join("audit.jsonl")
}

fn append(log: &Path, action: &str, resource: &str) {
    let output = run_binary(
        log,
        &[
            "audit",
            "append",
            "--action",
            action,
            "--resource",
            resource,
        ],
    );
    assert!(
        output.status.success(),
        "append failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn appending_twice_yields_a_valid_chain() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);

    append(&log, "open", "case-1");
    append(&log, "close", "case-1");

    let output = run_binary(&log, &["audit", "verify-chain"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("valid"), "stdout: {stdout}");
    assert!(stdout.contains('2'), "stdout: {stdout}");
}

#[test]
fn verifying_a_missing_log_fails_naming_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);
    let output = run_binary(&log, &["audit", "verify-chain"]);

    // A missing file must not be reported as a valid empty chain: it means
    // the path is wrong or the log was deleted outright.
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(log.to_str().unwrap()),
        "stderr must name the missing log path: {stderr}"
    );
}

#[test]
fn showing_a_missing_log_fails_naming_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);
    let output = run_binary(&log, &["audit", "show"]);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains(log.to_str().unwrap()),
        "stderr must name the missing log path: {stderr}"
    );
}

#[test]
fn verifying_an_existing_empty_file_reports_an_empty_valid_chain() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);
    std::fs::write(&log, "").unwrap();

    let output = run_binary(&log, &["--json", "audit", "verify-chain"]);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["valid"], true);
    assert_eq!(report["entries"], 0);
}

#[test]
fn a_corrupted_line_makes_verify_chain_fail_naming_the_index() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);
    append(&log, "open", "case-1");
    append(&log, "close", "case-1");

    // Tamper with the second line: change its actor without touching the
    // stored chain value.
    let content = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<String> = content
        .lines()
        .enumerate()
        .map(|(i, line)| {
            if i == 1 {
                line.replace("\"actor\":\"tester\"", "\"actor\":\"mallory\"")
            } else {
                line.to_string()
            }
        })
        .collect();
    assert_ne!(lines.join("\n"), content.trim_end());
    std::fs::write(&log, lines.join("\n") + "\n").unwrap();

    let output = run_binary(&log, &["audit", "verify-chain"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("index 1"),
        "stderr must name the broken index: {stderr}"
    );

    // The JSON report carries the same index on standard output.
    let output = run_binary(&log, &["--json", "audit", "verify-chain"]);
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let report: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(report["valid"], false);
    assert_eq!(report["first_broken_index"], 1);
}

#[test]
fn show_lists_the_appended_entries_as_json() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);
    append(&log, "open", "case-1");
    append(&log, "close", "case-2");

    let output = run_binary(&log, &["--json", "audit", "show"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let entries: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    let entries = entries.as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["sequence"], 0);
    assert_eq!(entries[0]["actor"], "tester");
    assert_eq!(entries[0]["action"], "open");
    assert_eq!(entries[0]["resource"], "case-1");
    assert_eq!(entries[1]["sequence"], 1);
    assert_eq!(entries[1]["resource"], "case-2");
    assert_eq!(entries[1]["chain"].as_str().unwrap().len(), 64);
}

#[test]
fn append_json_reports_the_stored_entry() {
    let dir = tempfile::tempdir().unwrap();
    let log = log_path(&dir);

    let output = run_binary(
        &log,
        &[
            "--json",
            "audit",
            "append",
            "--action",
            "open",
            "--resource",
            "case-1",
        ],
    );
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    let entry: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(entry["sequence"], 0);
    assert_eq!(entry["actor"], "tester");
    assert!(entry["timestamp"].as_str().unwrap().ends_with('Z'));
}
