//! Local and central directory headers, held to the exact field values the
//! server's stored ZIP writer emits.

use super::bytes::{u16_at, u32_at};
use super::limits::{name_is_safe, MAX_NAME_LEN};
use crate::report::Finding;

/// Local file header signature ("PK\x03\x04").
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;

/// Central directory header signature ("PK\x01\x02").
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;

/// Fixed part of a local header.
pub(super) const LOCAL_LEN: usize = 30;

/// Fixed part of a central header.
const CENTRAL_LEN: usize = 46;

/// Version needed to extract: 1.0, enough for stored entries.
const VERSION_NEEDED: u16 = 10;

/// Version made by: 2.0 with MS-DOS attribute style.
const VERSION_MADE_BY: u16 = 20;

/// DOS date 1980-01-01; the writer always pairs it with time 0.
const DOS_EPOCH_DATE: u16 = 0x0021;

/// General purpose flag: the entry is encrypted.
const FLAG_ENCRYPTED: u16 = 0x0001;

/// General purpose flag: sizes and CRC follow the data in a descriptor.
const FLAG_DATA_DESCRIPTOR: u16 = 0x0008;

/// The fields every admitted header describes, local or central.
#[derive(Debug)]
pub(super) struct Header<'a> {
    pub name: &'a [u8],
    pub crc: u32,
    pub size: u32,
    /// Where the local header starts; for a local header, its own offset.
    pub offset: u32,
    /// Bytes from the start of this header to the end of its name.
    pub len: usize,
}

impl Header<'_> {
    /// True when both headers describe the same entry: name, CRC-32, size
    /// and local header offset.
    pub(super) fn describes(&self, other: &Header<'_>) -> bool {
        self.name == other.name
            && self.crc == other.crc
            && self.size == other.size
            && self.offset == other.offset
    }
}

/// Reads the local header at `at`, which must end at or before `limit`.
pub(super) fn read_local(bytes: &[u8], at: usize, limit: usize) -> Result<Header<'_>, Finding> {
    let detail = || format!("local header at offset {at}");
    if at.checked_add(LOCAL_LEN).is_none_or(|end| end > limit) {
        return Err(unsupported("zip.layout", detail()));
    }
    if u32_at(bytes, at) != LOCAL_SIGNATURE {
        return Err(unsupported("zip.local_header", detail()));
    }
    check_fields(
        &Fields {
            version_needed: u16_at(bytes, at + 4),
            flags: u16_at(bytes, at + 6),
            method: u16_at(bytes, at + 8),
            time: u16_at(bytes, at + 10),
            date: u16_at(bytes, at + 12),
            extra_len: u16_at(bytes, at + 28),
        },
        &detail,
    )?;
    let sizes = [u32_at(bytes, at + 18), u32_at(bytes, at + 22)];
    let size = check_sizes(sizes, None, &detail)?;
    let name = read_name(
        bytes,
        at + LOCAL_LEN,
        u16_at(bytes, at + 26),
        limit,
        &detail,
    )?;
    Ok(Header {
        name,
        crc: u32_at(bytes, at + 14),
        size,
        offset: at as u32,
        len: LOCAL_LEN + name.len(),
    })
}

/// Reads the central header at `at`, which must end at or before `limit`.
pub(super) fn read_central(bytes: &[u8], at: usize, limit: usize) -> Result<Header<'_>, Finding> {
    let detail = || format!("central header at offset {at}");
    if at.checked_add(CENTRAL_LEN).is_none_or(|end| end > limit) {
        return Err(unsupported("zip.layout", detail()));
    }
    if u32_at(bytes, at) != CENTRAL_SIGNATURE {
        return Err(unsupported("zip.central_header", detail()));
    }
    if u16_at(bytes, at + 4) != VERSION_MADE_BY {
        return Err(unsupported("zip.version", detail()));
    }
    check_fields(
        &Fields {
            version_needed: u16_at(bytes, at + 6),
            flags: u16_at(bytes, at + 8),
            method: u16_at(bytes, at + 10),
            time: u16_at(bytes, at + 12),
            date: u16_at(bytes, at + 14),
            extra_len: u16_at(bytes, at + 30),
        },
        &detail,
    )?;
    if u16_at(bytes, at + 32) != 0 {
        return Err(unsupported("zip.comment", detail()));
    }
    if u16_at(bytes, at + 34) != 0 {
        return Err(unsupported("zip.multi_disk", detail()));
    }
    if u16_at(bytes, at + 36) != 0 || u32_at(bytes, at + 38) != 0 {
        return Err(unsupported("zip.attributes", detail()));
    }
    let offset = u32_at(bytes, at + 42);
    let sizes = [u32_at(bytes, at + 20), u32_at(bytes, at + 24)];
    let size = check_sizes(sizes, Some(offset), &detail)?;
    let name = read_name(
        bytes,
        at + CENTRAL_LEN,
        u16_at(bytes, at + 28),
        limit,
        &detail,
    )?;
    Ok(Header {
        name,
        crc: u32_at(bytes, at + 16),
        size,
        offset,
        len: CENTRAL_LEN + name.len(),
    })
}

/// Fields that local and central headers share and the writer fixes.
struct Fields {
    version_needed: u16,
    flags: u16,
    method: u16,
    time: u16,
    date: u16,
    extra_len: u16,
}

fn check_fields(fields: &Fields, detail: &impl Fn() -> String) -> Result<(), Finding> {
    let code = if fields.version_needed != VERSION_NEEDED {
        "zip.version"
    } else if fields.flags & FLAG_ENCRYPTED != 0 {
        "zip.encrypted"
    } else if fields.flags & FLAG_DATA_DESCRIPTOR != 0 {
        "zip.data_descriptor"
    } else if fields.flags != 0 {
        "zip.flags"
    } else if fields.method != 0 {
        "zip.method"
    } else if fields.time != 0 || fields.date != DOS_EPOCH_DATE {
        "zip.timestamp"
    } else if fields.extra_len != 0 {
        "zip.extra_field"
    } else {
        return Ok(());
    };
    Err(unsupported(code, detail()))
}

/// Checks the compressed and uncompressed sizes, and the local offset of a
/// central header, against ZIP64 sentinels; a stored entry has equal sizes.
fn check_sizes(
    [compressed, size]: [u32; 2],
    offset: Option<u32>,
    detail: &impl Fn() -> String,
) -> Result<u32, Finding> {
    if compressed == u32::MAX || size == u32::MAX || offset == Some(u32::MAX) {
        return Err(unsupported("zip.zip64", detail()));
    }
    if compressed != size {
        return Err(unsupported("zip.stored_size", detail()));
    }
    Ok(size)
}

fn read_name<'a>(
    bytes: &'a [u8],
    at: usize,
    len: u16,
    limit: usize,
    detail: &impl Fn() -> String,
) -> Result<&'a [u8], Finding> {
    let len = usize::from(len);
    if len == 0 || len > MAX_NAME_LEN {
        return Err(unsupported("zip.entry_name", detail()));
    }
    if at + len > limit {
        return Err(unsupported("zip.layout", detail()));
    }
    let name = &bytes[at..at + len];
    if !name_is_safe(name) {
        return Err(unsupported("zip.entry_name", detail()));
    }
    Ok(name)
}

fn unsupported(code: &'static str, detail: String) -> Finding {
    Finding::not_supported(code, detail)
}
