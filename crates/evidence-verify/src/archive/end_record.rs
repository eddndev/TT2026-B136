//! The end of central directory record: exactly one, last in the archive,
//! without comment, describing one disk and a directory that ends where the
//! record starts.

use super::bytes::{u16_at, u32_at};
use super::limits::MAX_ENTRIES;
use crate::report::Finding;

/// End record signature ("PK\x05\x06").
const SIGNATURE: u32 = 0x0605_4b50;

/// Fixed length of the record without comment.
pub(super) const LEN: usize = 22;

/// Longest comment the record can announce.
const MAX_COMMENT: usize = u16::MAX as usize;

/// What the end record says about the rest of the archive.
#[derive(Debug)]
pub(super) struct EndRecord {
    pub entries: usize,
    pub directory_offset: usize,
    pub directory_end: usize,
}

/// Reads and checks the end record of `bytes`.
pub(super) fn read(bytes: &[u8]) -> Result<EndRecord, Finding> {
    let Some(at) = bytes.len().checked_sub(LEN) else {
        return Err(unsupported("zip.end_record", "shorter than an end record"));
    };
    if u32_at(bytes, at) != SIGNATURE {
        return Err(misplaced(bytes, at));
    }
    if u16_at(bytes, at + 20) != 0 {
        return Err(unsupported("zip.comment", "archive comment"));
    }
    let disk = u16_at(bytes, at + 4);
    let directory_disk = u16_at(bytes, at + 6);
    let entries_on_disk = u16_at(bytes, at + 8);
    let entries = u16_at(bytes, at + 10);
    let directory_size = u32_at(bytes, at + 12);
    let directory_offset = u32_at(bytes, at + 16);
    if entries == u16::MAX
        || entries_on_disk == u16::MAX
        || directory_size == u32::MAX
        || directory_offset == u32::MAX
    {
        return Err(unsupported("zip.zip64", "end record"));
    }
    if disk != 0 || directory_disk != 0 {
        return Err(unsupported("zip.multi_disk", "end record"));
    }
    if entries_on_disk != entries {
        return Err(unsupported("zip.entry_count", "end record counts differ"));
    }
    if entries == 0 {
        return Err(unsupported("zip.empty", "no entries"));
    }
    let entries = usize::from(entries);
    if entries > MAX_ENTRIES {
        return Err(unsupported("zip.too_many_entries", entries.to_string()));
    }
    let directory_end = u64::from(directory_offset) + u64::from(directory_size);
    if directory_end != at as u64 {
        return Err(unsupported(
            "zip.layout",
            "the central directory does not end at the end record",
        ));
    }
    Ok(EndRecord {
        entries,
        directory_offset: directory_offset as usize,
        directory_end: at,
    })
}

/// The finding for an archive whose last 22 bytes are not an end record:
/// a record followed by exactly its announced comment is a comment, anything
/// else has no usable end record.
fn misplaced(bytes: &[u8], last: usize) -> Finding {
    let earliest = last.saturating_sub(MAX_COMMENT);
    let commented = (earliest..last).rev().find(|&at| {
        u32_at(bytes, at) == SIGNATURE && usize::from(u16_at(bytes, at + 20)) == last - at
    });
    match commented {
        Some(_) => unsupported("zip.comment", "archive comment"),
        None => unsupported("zip.end_record", "no end record at the end"),
    }
}

fn unsupported(code: &'static str, detail: impl Into<String>) -> Finding {
    Finding::not_supported(code, detail)
}
