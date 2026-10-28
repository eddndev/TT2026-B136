//! Admission of an evidence package: the stored ZIP exactly as the server's
//! writer produces it, and nothing else.
//!
//! The admitted shape is method 0, flags 0, version needed 10, made by 20,
//! DOS date 1980-01-01 at time 0, no extra fields, comments, data
//! descriptors, encryption, ZIP64 or attributes, local entries contiguous
//! from offset 0, then a central directory whose headers agree with them in
//! the same order, then a single end record. Names follow the server's safe
//! set and are unique byte for byte. Anything else, and any package beyond
//! the size limits, is not supported; a CRC-32 that does not match its
//! content fails the package. Rationale and limits:
//! docs/adr/0085-in-browser-evidence-validation.md.

mod bytes;
mod crc;
mod end_record;
mod headers;
mod limits;

#[cfg(test)]
mod tests;

use crate::report::Finding;
use headers::Header;
use limits::{entry_limit, DOCUMENT_LIMIT, MAX_TOTAL};

/// One admitted entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The name, within the safe ASCII set.
    pub name: String,
    /// The stored content, whose CRC-32 matched.
    pub data: Vec<u8>,
}

/// The entries of an admitted package, in archive order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    pub entries: Vec<Entry>,
}

impl Archive {
    /// The content of the entry named exactly `name`.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.entries
            .iter()
            .find(|entry| entry.name == name)
            .map(|entry| entry.data.as_slice())
    }

    /// Every entry name, in archive order.
    pub fn names(&self) -> Vec<&str> {
        self.entries
            .iter()
            .map(|entry| entry.name.as_str())
            .collect()
    }
}

/// Admits `bytes` as an evidence package.
///
/// # Errors
///
/// A not supported finding when the archive departs from the writer's shape
/// or exceeds a limit; a failed `zip.crc_mismatch` finding when it has that
/// shape but an entry's content does not match its CRC-32.
pub fn admit(bytes: &[u8]) -> Result<Archive, Finding> {
    let end = end_record::read(bytes)?;
    let locals = read_locals(bytes, end.entries, end.directory_offset)?;
    let mut at = end.directory_offset;
    for local in &locals {
        let central = headers::read_central(bytes, at, end.directory_end)?;
        if !central.describes(&local.header) {
            let detail = String::from_utf8_lossy(local.header.name).into_owned();
            return Err(Finding::not_supported("zip.header_mismatch", detail));
        }
        at += central.len;
    }
    if at != end.directory_end {
        return Err(Finding::not_supported(
            "zip.layout",
            "bytes after the last central header",
        ));
    }
    let entries = locals
        .into_iter()
        .map(verified_entry)
        .collect::<Result<_, _>>()?;
    Ok(Archive { entries })
}

/// A local header and the content that follows it.
struct Local<'a> {
    header: Header<'a>,
    data: &'a [u8],
}

/// Walks the local entries from offset 0 until the central directory, which
/// they must reach exactly, enforcing names, uniqueness and size limits. The
/// per-entry limit depends on the other names, so the walk caps each entry at
/// the largest limit and the name-aware limits are applied once every name
/// is known.
fn read_locals(bytes: &[u8], expected: usize, directory: usize) -> Result<Vec<Local<'_>>, Finding> {
    let mut locals: Vec<Local<'_>> = Vec::with_capacity(expected);
    let mut at = 0;
    let mut total = 0u64;
    while at < directory {
        let header = headers::read_local(bytes, at, directory)?;
        if locals.len() == expected {
            return Err(Finding::not_supported(
                "zip.entry_count",
                "more local entries than the end record counts",
            ));
        }
        let name = String::from_utf8_lossy(header.name).into_owned();
        if locals.iter().any(|seen| seen.header.name == header.name) {
            return Err(Finding::not_supported("zip.duplicate_name", name));
        }
        let size = u64::from(header.size);
        if size > DOCUMENT_LIMIT {
            return Err(Finding::not_supported("zip.entry_too_large", name));
        }
        total += size;
        if total > MAX_TOTAL {
            return Err(Finding::not_supported("zip.total_too_large", name));
        }
        let start = at + header.len;
        let end = start + header.size as usize;
        if end > directory {
            return Err(Finding::not_supported("zip.layout", name));
        }
        locals.push(Local {
            data: &bytes[start..end],
            header,
        });
        at = end;
    }
    if locals.len() != expected {
        return Err(Finding::not_supported(
            "zip.entry_count",
            "fewer local entries than the end record counts",
        ));
    }
    let names: Vec<&[u8]> = locals.iter().map(|local| local.header.name).collect();
    if let Some(local) = locals
        .iter()
        .find(|local| u64::from(local.header.size) > entry_limit(local.header.name, &names))
    {
        let name = String::from_utf8_lossy(local.header.name).into_owned();
        return Err(Finding::not_supported("zip.entry_too_large", name));
    }
    Ok(locals)
}

fn verified_entry(local: Local<'_>) -> Result<Entry, Finding> {
    let name = String::from_utf8_lossy(local.header.name).into_owned();
    if crc::crc32(local.data) != local.header.crc {
        return Err(Finding::failed("zip.crc_mismatch", name));
    }
    Ok(Entry {
        name,
        data: local.data.to_vec(),
    })
}
