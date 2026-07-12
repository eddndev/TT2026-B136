//! The stored ZIP writer produces archives that an independent tool
//! accepts: the system `unzip` binary tests them, extracts them, and
//! detects a corrupted member through the stored CRC-32.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use domain::crypto::archive::{ArchiveEntry, ArchiveWriter};
use infrastructure::StoredZipWriter;

fn entry(name: &str, content: &[u8]) -> ArchiveEntry {
    ArchiveEntry::new(name, content.to_vec()).unwrap()
}

/// Every byte value once, so extraction is compared over binary data.
fn all_byte_values() -> Vec<u8> {
    (0u8..=255).collect()
}

fn sample_entries() -> Vec<ArchiveEntry> {
    vec![
        entry("document.txt", b"cuerpo del documento de prueba\n"),
        entry("document.txt.sig", &all_byte_values()),
        entry("INSTRUCCIONES.md", b"# Instrucciones\n"),
        entry("vacio.bin", b""),
    ]
}

fn unzip(args: &[&str], dir: &Path) -> Output {
    Command::new("unzip")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("the unzip binary is runnable")
}

#[test]
fn unzip_accepts_the_archive_and_extraction_restores_every_byte() {
    let dir = tempfile::tempdir().unwrap();
    let archive = StoredZipWriter::new()
        .write_archive(&sample_entries())
        .unwrap();
    fs::write(dir.path().join("evidence.zip"), &archive).unwrap();

    let test = unzip(&["-t", "evidence.zip"], dir.path());
    assert!(
        test.status.success(),
        "unzip -t rejected the archive: {}{}",
        String::from_utf8_lossy(&test.stdout),
        String::from_utf8_lossy(&test.stderr)
    );

    let extract = unzip(&["-o", "evidence.zip", "-d", "out"], dir.path());
    assert!(
        extract.status.success(),
        "unzip extraction failed: {}{}",
        String::from_utf8_lossy(&extract.stdout),
        String::from_utf8_lossy(&extract.stderr)
    );

    for entry in sample_entries() {
        let restored = fs::read(dir.path().join("out").join(entry.name()))
            .unwrap_or_else(|err| panic!("{} was not extracted: {err}", entry.name()));
        assert_eq!(
            restored,
            entry.content(),
            "{} did not survive the round trip byte for byte",
            entry.name()
        );
    }
}

#[test]
fn unzip_detects_a_flipped_byte_through_the_crc() {
    let dir = tempfile::tempdir().unwrap();
    let mut archive = StoredZipWriter::new()
        .write_archive(&sample_entries())
        .unwrap();

    // Flip one byte inside the first entry's stored content, which
    // starts right after the 30-byte local header and the entry name.
    let content_start = 30 + "document.txt".len();
    archive[content_start + 3] ^= 0xff;
    fs::write(dir.path().join("tampered.zip"), &archive).unwrap();

    let test = unzip(&["-t", "tampered.zip"], dir.path());
    assert!(
        !test.status.success(),
        "unzip -t must reject a member whose bytes were altered"
    );
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&test.stdout),
        String::from_utf8_lossy(&test.stderr)
    );
    assert!(
        report.contains("bad CRC") || report.contains("CRC error"),
        "the rejection should name the CRC check, got: {report}"
    );
}
