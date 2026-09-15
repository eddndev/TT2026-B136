#![cfg(target_os = "linux")]

use base64::Engine;
use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn framed(contents: &[&[u8]]) -> Vec<u8> {
    let mut input = b"DFMT1".to_vec();
    input.push(contents.len() as u8);
    for content in contents {
        input.extend_from_slice(&(content.len() as u32).to_be_bytes());
        input.extend_from_slice(content);
    }
    input
}

#[test]
fn one_native_worker_admits_two_complete_pdf_documents() {
    let pdf = include_bytes!("../../infrastructure/tests/fixtures/stage-support.pdf");
    let result = worker(&framed(&[pdf, pdf]));
    assert!(
        result.status.success(),
        "worker status: {:?}",
        result.status
    );
    assert_eq!(result.stdout, b"DFMR1\x00\x02\x00\x00");
    assert!(result.stderr.is_empty());
}

#[test]
fn one_native_worker_admits_pdf_and_a_docx_from_an_independent_producer() {
    let pdf = include_bytes!("../../infrastructure/tests/fixtures/stage-support.pdf");
    let docx = include_bytes!(
        "../../infrastructure/src/document_formats/docx/tests/fixtures/producer.docx"
    );
    let result = worker(&framed(&[pdf, docx]));
    assert!(
        result.status.success(),
        "worker status: {:?}",
        result.status
    );
    assert_eq!(result.stdout, b"DFMR1\x00\x02\x00\x01");
    assert!(result.stderr.is_empty());
}

#[test]
fn kernel_resource_limits_are_installed_before_reading_document_bytes() {
    use std::{
        fs, thread,
        time::{Duration, Instant},
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .args(["--document-format-worker", "/bin/true"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let path = format!("/proc/{}/limits", child.id());
    let deadline = Instant::now() + Duration::from_secs(2);
    let expected = [
        ("Max cpu time", "5"),
        ("Max address space", "268435456"),
        ("Max file size", "0"),
        ("Max core file size", "0"),
    ];
    let observed = loop {
        let text = fs::read_to_string(&path).unwrap_or_default();
        let installed = expected.iter().all(|(label, limit)| {
            text.lines()
                .find_map(|line| line.strip_prefix(label))
                .is_some_and(|rest| rest.split_whitespace().take(2).eq([*limit, *limit]))
        });
        if installed || Instant::now() >= deadline {
            break installed;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let _ = child.kill();
    child.wait().unwrap();
    assert!(
        observed,
        "worker must install one budget before accepting input"
    );
}

#[test]
fn encrypted_pdf_with_an_empty_open_password_is_rejected() {
    let encoded = include_str!("../../infrastructure/tests/fixtures/stage-encrypted-pdf.b64")
        .lines()
        .collect::<String>();
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let result = worker(&framed(&[&pdf]));
    assert!(
        result.status.success(),
        "worker status: {:?}",
        result.status
    );
    assert_eq!(result.stdout, b"DFMR1\x01\x00");
    assert!(result.stderr.is_empty());
}

fn worker(input: &[u8]) -> Output {
    let library = std::env::var_os("TT_TEST_QPDF_LIBRARY")
        .expect("TT_TEST_QPDF_LIBRARY must identify the reviewed qpdf library");
    let mut child = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .arg("--document-format-worker")
        .arg(library)
        .env("RUST_LOG", "trace")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let _ = child.stdin.take().unwrap().write_all(input);
    child.wait_with_output().unwrap()
}

#[test]
fn unsupported_content_returns_only_a_bounded_opaque_result() {
    let content = b"private file contents";
    let mut input = b"DFMT1\x01".to_vec();
    input.extend_from_slice(&(content.len() as u32).to_be_bytes());
    input.extend_from_slice(content);
    let result = worker(&input);
    assert!(
        result.status.success(),
        "worker status: {:?}",
        result.status
    );
    assert_eq!(result.stdout, b"DFMR1\x01\x00");
    assert!(result.stderr.is_empty());
}

#[test]
fn an_oversized_declared_payload_is_rejected_before_reading_it() {
    let mut input = b"DFMT1\x01".to_vec();
    input.extend_from_slice(&(16 * 1024 * 1024_u32 + 1).to_be_bytes());
    let result = worker(&input);
    assert!(
        result.status.success(),
        "worker status: {:?}",
        result.status
    );
    assert_eq!(result.stdout, b"DFMR1\x02\x00");
    assert!(result.stderr.is_empty());
}

#[test]
fn invalid_batch_counts_do_not_start_a_parser() {
    for count in [0, 3, 255] {
        let mut input = b"DFMT1".to_vec();
        input.push(count);
        let result = worker(&input);
        assert!(
            result.status.success(),
            "worker status: {:?}",
            result.status
        );
        assert_eq!(result.stdout, b"DFMR1\x04\x00");
        assert!(result.stderr.is_empty());
    }
}

#[test]
fn worker_exit_flushes_its_result_without_running_profile_file_writers() {
    let directory = tempfile::tempdir().unwrap();
    let profile = directory.path().join("worker.profraw");
    let mut child = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
        .args(["--document-format-worker", "/bin/true"])
        .env("LLVM_PROFILE_FILE", &profile)
        .current_dir(directory.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"DFMT1\x00").unwrap();
    let result = child.wait_with_output().unwrap();
    assert!(
        result.status.success(),
        "worker status: {:?}",
        result.status
    );
    assert_eq!(result.stdout, b"DFMR1\x04\x00");
    assert!(result.stderr.is_empty());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[test]
fn native_docx_expansion_budget_is_shared_by_the_entire_batch() {
    let encoded = include_str!("fixtures/expanded-budget-docx.b64")
        .lines()
        .collect::<String>();
    let docx = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let single = worker(&framed(&[&docx]));
    assert!(
        single.status.success(),
        "worker status: {:?}",
        single.status
    );
    assert_eq!(single.stdout, b"DFMR1\x00\x01\x01");
    assert!(single.stderr.is_empty());
    let pair = worker(&framed(&[&docx, &docx]));
    assert!(pair.status.success(), "worker status: {:?}", pair.status);
    assert_eq!(pair.stdout, b"DFMR1\x03\x00");
    assert!(pair.stderr.is_empty());
}
