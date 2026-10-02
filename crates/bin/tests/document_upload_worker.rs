//! Private admission workers apply limits before reading hostile input.

#![cfg(target_os = "linux")]

use application::{
    documents::{DocumentUploadAdmission, DocumentUploadError},
    ApplicationError,
};
use infrastructure::document_admission::IsolatedDocumentUploadAdmission;
use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BINARY: &str = env!("CARGO_BIN_EXE_despacho-cli");
fn pair(limits: &str, label: &str) -> Option<(u64, u64)> {
    let line = limits.lines().find_map(|line| line.strip_prefix(label))?;
    let mut fields = line.split_whitespace();
    Some((fields.next()?.parse().ok()?, fields.next()?.parse().ok()?))
}
fn check_limits(limits: &str, cpu: u64, memory: u64) {
    assert_eq!(pair(limits, "Max cpu time"), Some((cpu, cpu)));
    assert_eq!(pair(limits, "Max address space"), Some((memory, memory)));
    assert_eq!(pair(limits, "Max core file size"), Some((0, 0)));
    assert_eq!(pair(limits, "Max file size"), Some((0, 0)));
}
struct Worker(Option<Child>);
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn inspection_limits_are_installed_while_input_is_still_pending() {
    let child = Command::new(BINARY)
        .args(["--document-admission-worker", "/unused-qpdf-library"])
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let mut worker = Worker(Some(child));
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let limits = fs::read_to_string(format!("/proc/{pid}/limits")).unwrap();
        if pair(&limits, "Max address space") == Some((256 * 1024 * 1024, 256 * 1024 * 1024))
            && pair(&limits, "Max cpu time") == Some((5, 5))
            && pair(&limits, "Max core file size") == Some((0, 0))
            && pair(&limits, "Max file size") == Some((0, 0))
        {
            check_limits(&limits, 5, 256 * 1024 * 1024);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "inspection limits were not installed before input"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let mut child = worker.0.take().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"plain text\n").unwrap();
    drop(stdin);
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
    assert_eq!(result.stdout, b"GADM1\0\x02");
    assert!(!PathBuf::from(format!("/proc/{pid}")).exists());
}

#[test]
fn probe_and_decode_exec_keep_their_distinct_cpu_and_native_memory_limits() {
    for (mode, cpu) in [("probe", 2), ("decode", 8)] {
        let result = Command::new(BINARY)
            .args([
                "--document-decoder-worker",
                mode,
                "/bin/cat",
                "/proc/self/limits",
            ])
            .env_clear()
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(result.status.success());
        check_limits(
            std::str::from_utf8(&result.stdout).unwrap(),
            cpu,
            512 * 1024 * 1024,
        );
    }
}

#[test]
fn unrelated_inherited_descriptor_is_closed_before_decoder_exec() {
    let directory = tempfile::tempdir().unwrap();
    let secret = directory.path().join("unrelated-input");
    fs::write(&secret, b"unrelated descriptor contents").unwrap();
    let result = Command::new("/bin/sh")
        .args([
            "-c",
            concat!(
                "exec 9<\"$1\"; test -e /proc/self/fd/9 || exit 4; ",
                "exec \"$2\" --document-decoder-worker probe /bin/sh -c ",
                "'test ! -e /proc/self/fd/9 || exit 9; printf sanitized'"
            ),
            "worker-test",
        ])
        .arg(secret)
        .arg(BINARY)
        .env_clear()
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(result.status.success());
    assert_eq!(result.stdout, b"sanitized");
}

#[test]
fn private_exec_configuration_failures_use_reserved_exit_status() {
    for arguments in [
        vec!["--document-decoder-worker", "unknown", "/bin/true"],
        vec!["--document-decoder-worker", "probe", "relative-program"],
        vec![
            "--document-decoder-worker",
            "decode",
            "/missing-document-decoder",
        ],
    ] {
        let result = Command::new(BINARY)
            .args(arguments)
            .env_clear()
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(125));
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn supervisor_rejects_truncated_malformed_and_extra_inspection_responses() {
    for (response, expected) in [
        ("GADM", DocumentUploadError::Unavailable),
        ("OTHER\\000\\002", DocumentUploadError::Unavailable),
        ("GADM1\\000\\377", DocumentUploadError::Unavailable),
        ("GADM1\\001\\007", DocumentUploadError::Unavailable),
        ("GADM1\\000\\002extra", DocumentUploadError::Limit),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("inspection-worker");
        fs::write(&executable, format!("#!/bin/sh\nprintf '{response}'\n")).unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
        let validator = IsolatedDocumentUploadAdmission::new(
            executable,
            "/bin/true".into(),
            "/bin/true".into(),
            "/bin/true".into(),
        )
        .unwrap();
        match validator.validate(b"plain text\n").unwrap_err() {
            ApplicationError::DocumentUpload(actual) => assert_eq!(actual, expected),
            other => panic!("unexpected inspection protocol error: {other}"),
        }
    }
}

#[test]
fn decoder_allocator_is_bounded_without_forwarding_caller_configuration() {
    for mode in ["probe", "decode"] {
        let result = Command::new(BINARY)
            .args(["--document-decoder-worker", mode, "/usr/bin/env"])
            .env_clear()
            .env("MALLOC_ARENA_MAX", "4096")
            .env("GLIBC_TUNABLES", "glibc.malloc.arena_max=4096")
            .env(
                "PRIVATE_DECODER_TEST_VALUE",
                "must-not-reach-native-process",
            )
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(result.status.success());
        assert_eq!(result.stdout, b"MALLOC_ARENA_MAX=2\n");
        assert!(result.stderr.is_empty());
    }
}
