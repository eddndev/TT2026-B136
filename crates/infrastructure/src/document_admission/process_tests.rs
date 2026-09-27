use super::process::run;
use application::{documents::DocumentUploadError, ApplicationError};
use rustix::fs::{fcntl_add_seals, memfd_create, MemfdFlags, SealFlags};
use std::{
    ffi::OsString,
    fs::{self, File},
    io::{Seek, Write},
    path::Path,
    thread,
    time::{Duration, Instant},
};

fn input(bytes: &[u8]) -> File {
    let fd = memfd_create(
        "document-process-test",
        MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
    )
    .unwrap();
    let mut file = File::from(fd);
    file.write_all(bytes).unwrap();
    file.rewind().unwrap();
    fcntl_add_seals(
        &file,
        SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL,
    )
    .unwrap();
    file
}
fn shell(script: &str, extra: &[&Path]) -> Vec<OsString> {
    let mut args = vec!["-c".into(), script.into(), "admission-test".into()];
    args.extend(extra.iter().map(|path| path.as_os_str().to_owned()));
    args
}
fn error(result: Result<Vec<u8>, ApplicationError>, expected: DocumentUploadError) {
    match result.unwrap_err() {
        ApplicationError::DocumentUpload(actual) => assert_eq!(actual, expected),
        other => panic!("unexpected supervisor error: {other}"),
    }
}
fn pid(path: &Path) -> u32 {
    fs::read_to_string(path).unwrap().trim().parse().unwrap()
}
fn stopped(pid: u32) {
    let until = Instant::now() + Duration::from_secs(1);
    loop {
        let status = fs::read_to_string(format!("/proc/{pid}/stat"));
        if let Err(failure) = &status {
            if failure.kind() == std::io::ErrorKind::NotFound {
                return;
            }
        }
        if let Ok(status) = status {
            let state = status.rsplit_once(") ").unwrap().1.as_bytes()[0];
            if matches!(state, b'Z' | b'X') {
                return;
            }
        }
        assert!(
            Instant::now() < until,
            "owned descendant {pid} is still executing"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn returns_exact_bounded_output_from_seekable_input() {
    let bytes = b"input\0with\xffbinary\n";
    assert_eq!(
        run(
            Path::new("/bin/sh"),
            &shell("exec /bin/cat", &[]),
            &input(bytes),
            Instant::now() + Duration::from_secs(2),
            bytes.len()
        )
        .unwrap(),
        bytes
    );
}

#[test]
fn accepts_empty_and_exact_limit_output_but_rejects_one_extra_byte() {
    let file = input(b"");
    for (script, size, accepted) in [
        ("exit 0", 0, true),
        ("printf 12345678", 8, true),
        ("printf 123456789", 8, false),
        ("printf x", 0, false),
    ] {
        let result = run(
            Path::new("/bin/sh"),
            &shell(script, &[]),
            &file,
            Instant::now() + Duration::from_secs(2),
            size,
        );
        if accepted {
            assert_eq!(result.unwrap().len(), size);
        } else {
            error(result, DocumentUploadError::Limit);
        }
    }
}

#[test]
fn child_nonzero_exit_is_invalid_and_never_returns_private_diagnostics() {
    error(
        run(
            Path::new("/bin/sh"),
            &shell(
                "printf private-document-bytes; printf secret-parser-detail >&2; exit 7",
                &[],
            ),
            &input(b""),
            Instant::now() + Duration::from_secs(2),
            64,
        ),
        DocumentUploadError::Invalid,
    );
}

#[test]
fn private_wrapper_startup_failure_is_unavailable() {
    error(
        run(
            Path::new("/bin/sh"),
            &shell("exit 125", &[]),
            &input(b""),
            Instant::now() + Duration::from_secs(2),
            16,
        ),
        DocumentUploadError::Unavailable,
    );
}

#[test]
fn cpu_and_kill_signals_are_resource_limit_failures() {
    for signal in ["KILL", "XCPU"] {
        error(
            run(
                Path::new("/bin/sh"),
                &shell(&format!("kill -{signal} $$"), &[]),
                &input(b""),
                Instant::now() + Duration::from_secs(2),
                16,
            ),
            DocumentUploadError::Limit,
        );
    }
}

#[test]
fn missing_executable_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    error(
        run(
            &dir.path().join("missing-executable"),
            &[],
            &input(b""),
            Instant::now() + Duration::from_secs(2),
            16,
        ),
        DocumentUploadError::Unavailable,
    );
}

#[test]
fn expired_shared_deadline_does_not_start_another_child() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("spawned");
    error(
        run(
            Path::new("/bin/sh"),
            &shell("printf started > \"$1\"", &[&marker]),
            &input(b""),
            Instant::now() - Duration::from_millis(1),
            16,
        ),
        DocumentUploadError::Limit,
    );
    assert!(!marker.exists());
}

#[test]
fn deadline_kills_group_and_reaps_the_direct_child() {
    let dir = tempfile::tempdir().unwrap();
    let leader = dir.path().join("leader.pid");
    let descendant = dir.path().join("descendant.pid");
    let start = Instant::now();
    error(
        run(
            Path::new("/bin/sh"),
            &shell(
                concat!(
                    "printf '%s' \"$$\" > \"$1\"; ",
                    "/bin/sleep 20 & printf '%s' \"$!\" > \"$2\"; wait"
                ),
                &[&leader, &descendant],
            ),
            &input(b""),
            start + Duration::from_millis(300),
            16,
        ),
        DocumentUploadError::Limit,
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(
        !Path::new(&format!("/proc/{}", pid(&leader))).exists(),
        "direct child must be reaped"
    );
    stopped(pid(&descendant));
}

#[test]
fn leader_exit_and_output_overflow_both_terminate_owned_descendants() {
    for (ending, expected, limit) in [
        ("printf ready; exit 0", None, 5),
        ("exit 7", Some(DocumentUploadError::Invalid), 5),
        ("printf overflow; wait", Some(DocumentUploadError::Limit), 2),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let leader = dir.path().join("leader.pid");
        let descendant = dir.path().join("descendant.pid");
        let script = format!(
            "printf '%s' \"$$\" > \"$1\"; /bin/sleep 20 & printf '%s' \"$!\" > \"$2\"; {ending}"
        );
        let start = Instant::now();
        let result = run(
            Path::new("/bin/sh"),
            &shell(&script, &[&leader, &descendant]),
            &input(b""),
            start + Duration::from_secs(3),
            limit,
        );
        if let Some(expected) = expected {
            error(result, expected);
        } else {
            assert_eq!(result.unwrap(), b"ready");
        }
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "must not wait for a descendant's pipe until the deadline"
        );
        assert!(
            !Path::new(&format!("/proc/{}", pid(&leader))).exists(),
            "direct child must be reaped"
        );
        stopped(pid(&descendant));
    }
}
