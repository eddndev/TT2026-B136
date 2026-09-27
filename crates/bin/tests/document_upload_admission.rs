//! General upload admission exercises real isolated parsers and decoders.

use application::{
    documents::{AdmittedDocumentFormat as Format, DocumentUploadAdmission, DocumentUploadError},
    ApplicationError,
};
use infrastructure::document_admission::IsolatedDocumentUploadAdmission;
use std::path::PathBuf;

fn validator() -> IsolatedDocumentUploadAdmission {
    let library = std::env::var_os("TT_TEST_QPDF_LIBRARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/lib64/libqpdf.so"));
    let program = |variable: &str, default: &str| {
        std::env::var_os(variable)
            .map(PathBuf::from)
            .unwrap_or_else(|| default.into())
    };
    IsolatedDocumentUploadAdmission::new(
        PathBuf::from(env!("CARGO_BIN_EXE_despacho-cli")),
        std::fs::canonicalize(library).unwrap(),
        program("TT_FFPROBE_PATH", "/opt/tt-media/bin/ffprobe"),
        program("TT_FFMPEG_PATH", "/opt/tt-media/bin/ffmpeg"),
    )
    .unwrap()
}

const PDF: &[u8] = include_bytes!("../../infrastructure/tests/fixtures/stage-support.pdf");
const DOCX: &[u8] =
    include_bytes!("../../infrastructure/src/document_formats/docx/tests/fixtures/producer.docx");
const PNG: &[u8] = include_bytes!("../../infrastructure/tests/fixtures/media-admission/tiny.png");
const JPEG: &[u8] = include_bytes!("../../infrastructure/tests/fixtures/media-admission/tiny.jpg");
const MP3: &[u8] = include_bytes!("../../infrastructure/tests/fixtures/media-admission/tiny.mp3");
const WAV: &[u8] = include_bytes!("../../infrastructure/tests/fixtures/media-admission/tiny.wav");
const MP4: &[u8] = include_bytes!("../../infrastructure/tests/fixtures/media-admission/tiny.mp4");
const FAST_MP4: &[u8] =
    include_bytes!("../../infrastructure/tests/fixtures/media-admission/tiny-faststart.mp4");

#[test]
fn admits_each_supported_format_with_real_bounded_decoding() {
    let validator = validator();
    for (bytes, format) in [
        (PDF, Format::Pdf),
        (DOCX, Format::Docx),
        (b"plain text\n".as_slice(), Format::Txt),
        (PNG, Format::Png),
        (JPEG, Format::Jpeg),
        (MP3, Format::Mp3),
        (WAV, Format::Wav),
        (MP4, Format::Mp4),
        (FAST_MP4, Format::Mp4),
    ] {
        assert_eq!(
            validator
                .validate(bytes)
                .unwrap_or_else(|error| panic!("{format:?}: {error}")),
            format,
            "{format:?}"
        );
    }
}

#[test]
fn rejects_truncated_native_formats_and_incomplete_media() {
    let validator = validator();
    for bytes in [PDF, DOCX, PNG, JPEG, MP3, WAV, MP4] {
        let result = validator.validate(&bytes[..bytes.len() / 2]);
        assert!(
            matches!(
                result,
                Err(ApplicationError::DocumentUpload(
                    DocumentUploadError::Invalid | DocumentUploadError::Unsupported
                ))
            ),
            "{result:?}"
        );
    }
    let mut trailing = PNG.to_vec();
    trailing.push(0);
    assert!(matches!(
        validator.validate(&trailing),
        Err(ApplicationError::DocumentUpload(
            DocumentUploadError::Invalid
        ))
    ));
}

#[test]
fn rejects_unknown_binary_and_input_above_the_admission_limit() {
    let validator = validator();
    for bytes in [b"MZ\x00\xff".as_slice(), b"GIF89a\x00", b""] {
        assert!(validator.validate(bytes).is_err());
    }
    assert!(matches!(
        validator.validate(&vec![b'x'; 16 * 1024 * 1024 + 1]),
        Err(ApplicationError::DocumentUpload(DocumentUploadError::Limit))
    ));
}

#[test]
fn validates_decoder_configuration_before_serving_uploads() {
    validator().check_configuration().unwrap();
    assert!(IsolatedDocumentUploadAdmission::new(
        PathBuf::from(env!("CARGO_BIN_EXE_despacho-cli")),
        PathBuf::from("/bin/true"),
        PathBuf::from("/missing/ffprobe"),
        PathBuf::from("/bin/true"),
    )
    .is_err());
}

#[test]
fn rejects_damaged_compressed_payload_after_its_structure_passes() {
    let validator = validator();
    let mut png = PNG.to_vec();
    let marker = png.windows(4).position(|part| part == b"IDAT").unwrap();
    let size = u32::from_be_bytes(png[marker - 4..marker].try_into().unwrap()) as usize;
    png[marker + 4..marker + 4 + size].fill(0);
    let mut crc = !0u32;
    for byte in &png[marker..marker + 4 + size] {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb88320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    png[marker + 4 + size..marker + 8 + size].copy_from_slice(&(!crc).to_be_bytes());
    let mut mp4 = MP4.to_vec();
    let marker = mp4.windows(4).position(|part| part == b"mdat").unwrap();
    let size = u32::from_be_bytes(mp4[marker - 4..marker].try_into().unwrap()) as usize;
    mp4[marker + 4..marker - 4 + size].fill(0);
    for (bytes, format) in [(png, 4), (mp4, 7)] {
        use std::{
            io::Write,
            process::{Command, Stdio},
        };
        let mut child = Command::new(env!("CARGO_BIN_EXE_despacho-cli"))
            .args(["--document-admission-worker", "/unused-library"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&bytes).unwrap();
        let result = child.wait_with_output().unwrap();
        assert!(result.status.success());
        assert_eq!(result.stdout, [b'G', b'A', b'D', b'M', b'1', 0, format]);
        assert!(
            matches!(
                validator.validate(&bytes),
                Err(ApplicationError::DocumentUpload(
                    DocumentUploadError::Invalid
                ))
            ),
            "structurally framed corruption must fail complete decoding"
        );
    }
}
