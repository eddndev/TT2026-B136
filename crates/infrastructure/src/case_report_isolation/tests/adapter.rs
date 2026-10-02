use super::{super::*, support::*};
use application::case_reports::*;
use std::{
    fs,
    io::{Read, Seek, Write},
    os::unix::fs::PermissionsExt,
};

#[test]
fn sealed_capture_preserves_exact_bytes_and_cannot_be_mutated_or_resized() {
    use rustix::fs::{fcntl_get_seals, SealFlags};
    let bytes = b"capture\0bytes\xff";
    let mut input = sealed::input(bytes).unwrap();
    assert_eq!(
        fcntl_get_seals(&input).unwrap(),
        SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL
    );
    let mut observed = Vec::new();
    input.read_to_end(&mut observed).unwrap();
    assert_eq!(observed, bytes);
    assert!(input.write_all(b"replacement").is_err());
    assert!(input.set_len(0).is_err());
    input.rewind().unwrap();
    error(
        sealed::input(&vec![0; MAX_REPORT_SNAPSHOT_BYTES + 1]),
        CaseReportError::CapacityExceeded,
    );
}

#[test]
fn absolute_worker_receives_only_private_arguments_cleared_environment_and_capture() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("worker");
    let captured = directory.path().join("captured.json");
    let response = directory.path().join("response");
    let value = snapshot();
    let context = protocol::Context {
        report_id: value.report_id,
        snapshot_digest: value.digest,
        format: CaseReportFormat::Csv,
    };
    fs::write(&response, frame(context, b"literal csv\r\n")).unwrap();
    fs::write(&executable, format!(concat!(
        "#!/bin/sh\n",
        "test \"$#\" -eq 2 && test \"$1\" = --case-report-render-worker && test \"$2\" = csv || exit 125\n",
        "test \"$(pwd)\" = / && test -z \"$HOME$DATABASE_URL$REDIS_URL$AWS_SECRET_ACCESS_KEY\" || exit 125\n",
        "/bin/cat > '{}'\n/bin/cat '{}'\n"), captured.display(), response.display())).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let renderer = IsolatedCaseReportRenderer::new(executable).unwrap();
    assert_eq!(
        renderer.render(&value, CaseReportFormat::Csv).unwrap(),
        b"literal csv\r\n"
    );
    let wire: serde_json::Value = serde_json::from_slice(&fs::read(captured).unwrap()).unwrap();
    assert_eq!(wire["report_id"], value.report_id.to_string());
    assert_eq!(wire["principal"]["email"], "owner@example.test");
}

#[test]
fn invalid_snapshot_is_rejected_before_starting_the_worker() {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("worker");
    let marker = directory.path().join("started");
    fs::write(
        &executable,
        format!("#!/bin/sh\nprintf started > '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let renderer = IsolatedCaseReportRenderer::new(executable).unwrap();
    let mut value = snapshot();
    value.digest = domain::crypto::Sha256Digest::from_array([99; 32]);
    error(
        renderer.render(&value, CaseReportFormat::Pdf),
        CaseReportError::StoredInconsistent(String::new()),
    );
    assert!(!marker.exists());
}
