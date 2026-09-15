use super::PdfLibrary;
use application::ApplicationError;
use base64::Engine;
use std::path::PathBuf;

fn library() -> PdfLibrary {
    let path = std::env::var_os("TT_TEST_QPDF_LIBRARY")
        .expect("TT_TEST_QPDF_LIBRARY must identify the reviewed qpdf library");
    PdfLibrary::open(&PathBuf::from(path)).unwrap()
}

fn pdf(pages: &str, page: &str) -> Vec<u8> {
    let objects = ["<< /Type /Catalog /Pages 2 0 R >>", pages, page];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{object}\nendobj\n", index + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes(),
    );
    out
}

fn valid() -> Vec<u8> {
    pdf(
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> >>",
    )
}

#[test]
fn admits_a_complete_unencrypted_page_tree() {
    library().validate(&valid()).unwrap();
}

#[test]
fn rejects_xref_inconsistencies_without_normalizing_them() {
    let library = library();
    let original = String::from_utf8(valid()).unwrap();
    let xref = original
        .split("startxref\n")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap();
    let variants = [
        original.replace("/Size 4", "/Size 5"),
        original.replace("xref\n0 4", "xref\n0 5"),
        original.replace("/Root 1 0 R", &format!("/Root 1 0 R /Prev {xref}")),
        original.replace("/Root 1 0 R", "/Root 1 0 R /Prev /Name"),
        original.replace(&format!("startxref\n{xref}"), "startxref\n0"),
        original[..original.len() - 35].to_owned(),
    ];
    for input in variants {
        assert!(matches!(
            library.validate(input.as_bytes()),
            Err(ApplicationError::StageSupportFormatRejected)
        ));
    }
}

#[test]
fn rejects_inconsistent_cyclic_and_empty_page_trees() {
    let library = library();
    let page = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>";
    for pages in [
        "<< /Type /Pages /Kids [3 0 R] /Count 2 >>",
        "<< /Type /Pages /Kids [2 0 R] /Count 1 >>",
        "<< /Type /Pages /Kids [] /Count 0 >>",
    ] {
        assert!(matches!(
            library.validate(&pdf(pages, page)),
            Err(ApplicationError::StageSupportFormatRejected)
        ));
    }
}

#[test]
fn missing_library_is_an_operational_failure_not_a_bad_document() {
    assert!(matches!(
        PdfLibrary::open(std::path::Path::new("/nonexistent/tt-qpdf-library")),
        Err(ApplicationError::InvalidConfiguration(_))
    ));
}

#[test]
fn admits_xref_streams_and_compressed_object_streams() {
    let bytes = stream_pdf();
    let original = bytes.clone();
    library().validate(&bytes).unwrap();
    assert_eq!(bytes, original);
}

#[test]
fn malformed_xref_stream_widths_are_rejected_without_recovery() {
    let bytes = stream_pdf();
    let needle = b"/W [ 1 1 1 ]";
    let position = bytes
        .windows(needle.len())
        .position(|value| value == needle)
        .unwrap();
    let mut damaged = bytes;
    damaged[position + 5] = b'2';
    assert!(matches!(
        library().validate(&damaged),
        Err(ApplicationError::StageSupportFormatRejected)
    ));
}

#[test]
fn admits_incremental_updates_with_an_exact_previous_xref() {
    let mut bytes = valid();
    let text = std::str::from_utf8(&bytes).unwrap();
    let previous = text
        .split("startxref\n")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_owned();
    let object = bytes.len();
    bytes.extend_from_slice(b"4 0 obj\n<< /Producer (Fixture) >>\nendobj\n");
    let xref = bytes.len();
    bytes.extend_from_slice(format!(
        "xref\n4 1\n{object:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R /Info 4 0 R /Prev {previous} >>\nstartxref\n{xref}\n%%EOF\n"
    ).as_bytes());
    let original = bytes.clone();
    library().validate(&bytes).unwrap();
    assert_eq!(bytes, original);
}

fn stream_pdf() -> Vec<u8> {
    let encoded = include_str!("fixtures/xref-stream.b64")
        .lines()
        .collect::<String>();
    base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap()
}
