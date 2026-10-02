//! Acceptance of the private report process before application initialization.
#![cfg(target_os = "linux")]

#[allow(dead_code)]
#[path = "../../infrastructure/tests/case_report_rendering/support.rs"]
mod support;

use application::case_reports::*;
use infrastructure::{case_report_isolation::IsolatedCaseReportRenderer, RingSha256Hasher};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const BINARY: &str = env!("CARGO_BIN_EXE_despacho-cli");
const FLAG: &str = "--case-report-render-worker";
struct Worker(Option<Child>);
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(child) = &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
fn pair(limits: &str, label: &str) -> Option<(u64, u64)> {
    let mut values = limits
        .lines()
        .find_map(|line| line.strip_prefix(label))?
        .split_whitespace();
    Some((values.next()?.parse().ok()?, values.next()?.parse().ok()?))
}
fn response(output: std::process::Output, status: u8, format: u8) {
    assert!(
        output.status.success(),
        "private worker failed: {:?}",
        output.status
    );
    assert!(
        output.stderr.is_empty(),
        "worker emitted private diagnostics"
    );
    let mut expected = vec![0; 95];
    expected[..5].copy_from_slice(b"TTRP1");
    expected[5] = status;
    expected[6] = format;
    assert_eq!(output.stdout, expected);
}

#[test]
fn isolated_pdf_and_csv_preserve_exact_bundled_renderer_bytes() {
    let mut snapshot = support::snapshot(1);
    snapshot.cases[0].title = "Audiencia de Jos\u{00e9}: prueba \"literal\"".into();
    snapshot.cases[0].reference = "=SUM(1,2)".into();
    snapshot.digest = case_report_snapshot_digest(&RingSha256Hasher, &snapshot).unwrap();
    let isolated = IsolatedCaseReportRenderer::new(BINARY.into()).unwrap();
    let direct =
        infrastructure::case_report_rendering::BoundedCaseReportRenderer::bundled().unwrap();
    for format in [CaseReportFormat::Pdf, CaseReportFormat::Csv] {
        let expected = direct.render(&snapshot, format).unwrap();
        let result = isolated.render(&snapshot, format).unwrap();
        assert_eq!(
            result, expected,
            "isolation changed the rendered format {format:?}"
        );
        assert!(!result.is_empty());
        assert!(result.len() <= MAX_REPORT_ARTIFACT_BYTES);
    }
}

#[test]
fn worker_installs_limits_and_closes_foreign_descriptors_before_reading_capture() {
    let directory = tempfile::tempdir().unwrap();
    let unrelated = directory.path().join("unrelated-descriptor");
    fs::write(&unrelated, b"private contents outside the report").unwrap();
    let child = Command::new("/bin/sh")
        .args([
            "-c",
            "exec 9<\"$1\"; exec \"$2\" --case-report-render-worker csv",
            "report-worker-test",
        ])
        .arg(&unrelated)
        .arg(BINARY)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    let mut worker = Worker(Some(child));
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let limits = fs::read_to_string(format!("/proc/{pid}/limits")).unwrap();
        if pair(&limits, "Max cpu time") == Some((15, 15))
            && pair(&limits, "Max address space") == Some((512 * 1024 * 1024, 512 * 1024 * 1024))
            && pair(&limits, "Max core file size") == Some((0, 0))
            && pair(&limits, "Max file size") == Some((0, 0))
            && !Path::new(&format!("/proc/{pid}/fd/9")).exists()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "private limits or descriptor cleanup were not applied before input"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let mut child = worker.0.take().unwrap();
    let mut input = child.stdin.take().unwrap();
    input.write_all(b"{not-json}").unwrap();
    drop(input);
    response(child.wait_with_output().unwrap(), 4, 2);
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
}

#[test]
fn oversized_sealed_input_returns_only_a_typed_capacity_failure() {
    use rustix::fs::{fcntl_add_seals, memfd_create, MemfdFlags, SealFlags};
    use std::{fs::File, io::Seek};
    let mut input = File::from(
        memfd_create(
            "report-worker-input",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .unwrap(),
    );
    input
        .write_all(&vec![b' '; MAX_REPORT_SNAPSHOT_BYTES + 1])
        .unwrap();
    input.rewind().unwrap();
    fcntl_add_seals(
        &input,
        SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL,
    )
    .unwrap();
    let output = Command::new(BINARY)
        .args([FLAG, "pdf"])
        .env_clear()
        .stdin(input)
        .output()
        .unwrap();
    response(output, 1, 1);
}

#[test]
fn private_entry_rejects_unknown_formats_missing_or_extra_arguments_without_cli_output() {
    for arguments in [vec![FLAG], vec![FLAG, "html"], vec![FLAG, "pdf", "extra"]] {
        let output = Command::new(BINARY)
            .args(arguments)
            .env_clear()
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(125));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
