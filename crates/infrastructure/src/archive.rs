//! Stored-entry ZIP writer for evidence archives.
//!
//! Writes the minimal subset of the ZIP format (PKWARE APPNOTE): one
//! local file header per entry with method 0 (stored, no compression),
//! a central directory, and the end-of-central-directory record, all
//! with correct CRC-32 checksums. Entries are small verification
//! artifacts, so compression buys nothing while a full ZIP dependency
//! would cost a large audited surface; the trade-off is recorded in
//! docs/adr/0006-evidence-package-format.md.
//!
//! Output is deterministic: entry order is the caller's and every
//! modification timestamp is fixed to the DOS epoch (1980-01-01), so
//! archiving the same entries twice yields identical bytes.

use domain::crypto::archive::{ArchiveEntry, ArchiveWriter};
use domain::DomainError;

/// Local file header signature ("PK\x03\x04").
const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4b50;

/// Central directory header signature ("PK\x01\x02").
const CENTRAL_HEADER_SIGNATURE: u32 = 0x0201_4b50;

/// End of central directory signature ("PK\x05\x06").
const END_RECORD_SIGNATURE: u32 = 0x0605_4b50;

/// Minimum ZIP feature version able to read stored entries (1.0).
const VERSION_NEEDED: u16 = 10;

/// Version-made-by field: 2.0, upper byte 0 (MS-DOS attribute style).
const VERSION_MADE_BY: u16 = 20;

/// DOS date for 1980-01-01, the earliest representable day; using a
/// fixed timestamp keeps the output deterministic.
const DOS_EPOCH_DATE: u16 = 0x0021;

/// [`ArchiveWriter`] emitting stored (uncompressed) ZIP archives.
#[derive(Debug, Clone, Copy, Default)]
pub struct StoredZipWriter;

impl StoredZipWriter {
    /// Creates the writer; it holds no state.
    pub fn new() -> Self {
        Self
    }
}

impl ArchiveWriter for StoredZipWriter {
    fn write_archive(&self, entries: &[ArchiveEntry]) -> Result<Vec<u8>, DomainError> {
        check_entries(entries)?;

        let mut archive = Vec::new();
        let mut directory = Vec::new();
        for entry in entries {
            let offset = fit_u32(archive.len(), "archive")?;
            let size = fit_u32(entry.content().len(), entry.name())?;
            let checksum = crc32(entry.content());

            write_local_header(&mut archive, entry, size, checksum);
            archive.extend_from_slice(entry.content());
            write_central_header(&mut directory, entry, size, checksum, offset);
        }

        let directory_offset = fit_u32(archive.len(), "archive")?;
        let directory_size = fit_u32(directory.len(), "central directory")?;
        archive.extend_from_slice(&directory);
        write_end_record(
            &mut archive,
            entries.len() as u16,
            directory_size,
            directory_offset,
        );
        Ok(archive)
    }
}

/// Rejects entry sets the format cannot hold: none at all, more than
/// the 16-bit entry count, or duplicate names.
fn check_entries(entries: &[ArchiveEntry]) -> Result<(), DomainError> {
    if entries.is_empty() {
        return Err(failure("an archive must hold at least one entry"));
    }
    if entries.len() > usize::from(u16::MAX) {
        return Err(failure("too many entries for one archive"));
    }
    for (index, entry) in entries.iter().enumerate() {
        let duplicated = entries[..index]
            .iter()
            .any(|earlier| earlier.name() == entry.name());
        if duplicated {
            return Err(failure(&format!("duplicate entry name: {}", entry.name())));
        }
    }
    Ok(())
}

fn write_local_header(out: &mut Vec<u8>, entry: &ArchiveEntry, size: u32, checksum: u32) {
    push_u32(out, LOCAL_HEADER_SIGNATURE);
    push_u16(out, VERSION_NEEDED);
    push_u16(out, 0); // general purpose flags
    push_u16(out, 0); // method 0: stored
    push_u16(out, 0); // modification time (midnight)
    push_u16(out, DOS_EPOCH_DATE);
    push_u32(out, checksum);
    push_u32(out, size); // compressed size (stored: equals the original)
    push_u32(out, size); // uncompressed size
    push_u16(out, entry.name().len() as u16);
    push_u16(out, 0); // extra field length
    out.extend_from_slice(entry.name().as_bytes());
}

fn write_central_header(
    out: &mut Vec<u8>,
    entry: &ArchiveEntry,
    size: u32,
    checksum: u32,
    local_offset: u32,
) {
    push_u32(out, CENTRAL_HEADER_SIGNATURE);
    push_u16(out, VERSION_MADE_BY);
    push_u16(out, VERSION_NEEDED);
    push_u16(out, 0); // general purpose flags
    push_u16(out, 0); // method 0: stored
    push_u16(out, 0); // modification time (midnight)
    push_u16(out, DOS_EPOCH_DATE);
    push_u32(out, checksum);
    push_u32(out, size);
    push_u32(out, size);
    push_u16(out, entry.name().len() as u16);
    push_u16(out, 0); // extra field length
    push_u16(out, 0); // comment length
    push_u16(out, 0); // disk number where the entry starts
    push_u16(out, 0); // internal attributes
    push_u32(out, 0); // external attributes
    push_u32(out, local_offset);
    out.extend_from_slice(entry.name().as_bytes());
}

fn write_end_record(out: &mut Vec<u8>, entries: u16, directory_size: u32, directory_offset: u32) {
    push_u32(out, END_RECORD_SIGNATURE);
    push_u16(out, 0); // this disk
    push_u16(out, 0); // disk holding the central directory
    push_u16(out, entries); // entries on this disk
    push_u16(out, entries); // entries in total
    push_u32(out, directory_size);
    push_u32(out, directory_offset);
    push_u16(out, 0); // comment length
}

/// CRC-32 as ZIP requires it (IEEE 802.3: reflected, polynomial
/// 0xEDB88320, initial value and final xor all ones). Bit-by-bit, which
/// is ample for archives assembled in memory.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let low_bit_mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & low_bit_mask);
        }
    }
    !crc
}

/// The classic format has 32-bit sizes and offsets; anything larger
/// would need ZIP64, which evidence packages never approach.
fn fit_u32(value: usize, what: &str) -> Result<u32, DomainError> {
    u32::try_from(value).map_err(|_| failure(&format!("{what} exceeds the 4 GiB format limit")))
}

fn failure(message: &str) -> DomainError {
    DomainError::ArchiveWriteFailure(message.to_string())
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, content: &[u8]) -> ArchiveEntry {
        ArchiveEntry::new(name, content.to_vec()).unwrap()
    }

    #[test]
    fn crc32_matches_the_standard_check_value() {
        // The check value from the CRC-32/ISO-HDLC specification.
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn the_archive_starts_with_a_local_header_and_ends_with_the_end_record() {
        let archive = StoredZipWriter::new()
            .write_archive(&[entry("a.txt", b"content")])
            .unwrap();
        assert_eq!(&archive[..4], b"PK\x03\x04");
        let end = archive.len() - 22; // fixed end-record length, no comment
        assert_eq!(&archive[end..end + 4], b"PK\x05\x06");
    }

    #[test]
    fn the_stored_content_appears_verbatim_after_its_header() {
        let archive = StoredZipWriter::new()
            .write_archive(&[entry("a.txt", b"verbatim body")])
            .unwrap();
        // Local header: 30 fixed bytes plus the 5-byte name.
        let start = 30 + "a.txt".len();
        assert_eq!(&archive[start..start + 13], b"verbatim body");
    }

    #[test]
    fn output_is_deterministic() {
        let entries = [entry("a.txt", b"one"), entry("b.bin", &[0u8, 255, 7])];
        let first = StoredZipWriter::new().write_archive(&entries).unwrap();
        let second = StoredZipWriter::new().write_archive(&entries).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn an_empty_entry_list_is_rejected() {
        let err = StoredZipWriter::new().write_archive(&[]).unwrap_err();
        assert!(matches!(err, DomainError::ArchiveWriteFailure(_)));
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let err = StoredZipWriter::new()
            .write_archive(&[entry("a.txt", b"x"), entry("a.txt", b"y")])
            .unwrap_err();
        assert!(
            err.to_string().contains("duplicate entry name: a.txt"),
            "unexpected error: {err}"
        );
    }
}
