use super::IsolatedDocumentFormatValidator;
use application::ApplicationError;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    time::{Duration, Instant},
};

fn script(source: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("worker.sh");
    fs::write(&path, format!("#!/bin/sh\n{source}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    (dir, path)
}

#[test]
fn timeout_terminates_and_reaps_a_worker_that_never_reads_stdin() {
    let (_dir, path) = script("printf '%s' \"$$\" > \"$0.pid\"; exec sleep 20");
    let pid_path = path.with_extension("sh.pid");
    let validator = IsolatedDocumentFormatValidator::new(path, PathBuf::from("/bin/true")).unwrap();
    let large = vec![42; 16 * 1024 * 1024];
    let start = Instant::now();
    let result = validator.run(&[&large], Duration::from_millis(300));
    assert!(matches!(
        result,
        Err(ApplicationError::StageSupportValidationLimit)
    ));
    assert!(start.elapsed() < Duration::from_secs(2));
    let pid: u32 = fs::read_to_string(pid_path).unwrap().parse().unwrap();
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "worker must be reaped before returning"
    );
}

#[test]
fn excessive_worker_output_cannot_grow_the_supervisor_buffer() {
    let (_dir, path) = script("exec yes private-document-text");
    let validator = IsolatedDocumentFormatValidator::new(path, PathBuf::from("/bin/true")).unwrap();
    let error = validator
        .run(&[b"file"], Duration::from_secs(1))
        .unwrap_err();
    assert!(matches!(error, ApplicationError::Port(_)));
    assert!(!error.to_string().contains("private-document-text"));
}

#[test]
fn early_rejection_is_preserved_when_the_worker_closes_its_input() {
    let (_dir, path) = script("printf 'DFMR1\\001\\000'");
    let validator = IsolatedDocumentFormatValidator::new(path, PathBuf::from("/bin/true")).unwrap();
    let large = vec![42; 16 * 1024 * 1024];
    assert!(matches!(
        validator.run(&[&large], Duration::from_secs(1)),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
}

#[test]
fn successful_result_must_match_the_input_cardinality() {
    let (_dir, path) = script("cat >/dev/null\nprintf 'DFMR1\\000\\001\\000'");
    let validator = IsolatedDocumentFormatValidator::new(path, PathBuf::from("/bin/true")).unwrap();
    assert!(matches!(
        validator.run(&[b"first", b"second"], Duration::from_secs(1)),
        Err(ApplicationError::Port(_))
    ));
}

#[test]
fn continuously_readable_input_does_not_pay_a_delay_per_pipe_chunk() {
    let (_dir, path) = script(concat!(
        "exec python3 -c 'import errno, fcntl, os\n",
        "while True:\n",
        " try: fcntl.fcntl(0, fcntl.F_SETPIPE_SZ, 4096); break\n",
        " except OSError as error:\n",
        "  if error.errno != errno.EBUSY: raise\n",
        "  os.read(0, 65536)\n",
        "while os.read(0, 65536): pass\n",
        "os.write(1, bytes([68, 70, 77, 82, 49, 0, 2, 0, 0]))'"
    ));
    let validator = IsolatedDocumentFormatValidator::new(path, PathBuf::from("/bin/true")).unwrap();
    let large = vec![42; 8 * 1024 * 1024];
    let result = validator.run(&[&large, &large], Duration::from_secs(2));
    assert_eq!(result.unwrap().len(), 2);
}
