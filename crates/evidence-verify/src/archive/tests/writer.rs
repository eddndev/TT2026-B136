//! Test-only reproduction of the server's stored ZIP writer.
//!
//! The layout copies `crates/infrastructure/src/archive.rs` field by field:
//! method 0, flags 0, version needed 10, made by 20, DOS date 1980-01-01 at
//! time 0, no extra fields or comments, external attributes 0, contiguous
//! local entries from offset 0, then the central directory and one end
//! record. Names are not validated here so that tests can build archives the
//! admission must refuse. The returned offsets let tests patch single fields.

/// Field offsets inside a local file header.
pub mod local {
    pub const SIGNATURE: usize = 0;
    pub const VERSION_NEEDED: usize = 4;
    pub const FLAGS: usize = 6;
    pub const METHOD: usize = 8;
    pub const TIME: usize = 10;
    pub const DATE: usize = 12;
    pub const CRC: usize = 14;
    pub const COMPRESSED_SIZE: usize = 18;
    pub const SIZE: usize = 22;
    pub const EXTRA_LEN: usize = 28;
    pub const NAME: usize = 30;
}

/// Field offsets inside a central directory header.
pub mod central {
    pub const SIGNATURE: usize = 0;
    pub const VERSION_MADE_BY: usize = 4;
    pub const VERSION_NEEDED: usize = 6;
    pub const FLAGS: usize = 8;
    pub const METHOD: usize = 10;
    pub const TIME: usize = 12;
    pub const DATE: usize = 14;
    pub const CRC: usize = 16;
    pub const COMPRESSED_SIZE: usize = 20;
    pub const SIZE: usize = 24;
    pub const EXTRA_LEN: usize = 30;
    pub const COMMENT_LEN: usize = 32;
    pub const DISK_START: usize = 34;
    pub const INTERNAL_ATTRIBUTES: usize = 36;
    pub const EXTERNAL_ATTRIBUTES: usize = 38;
    pub const LOCAL_OFFSET: usize = 42;
    pub const NAME: usize = 46;
}

/// Field offsets inside the end of central directory record.
pub mod end {
    pub const DISK: usize = 4;
    pub const DIRECTORY_DISK: usize = 6;
    pub const ENTRIES_ON_DISK: usize = 8;
    pub const ENTRIES: usize = 10;
    pub const DIRECTORY_SIZE: usize = 12;
    pub const DIRECTORY_OFFSET: usize = 16;
    pub const COMMENT_LEN: usize = 20;
}

const LOCAL_HEADER_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_HEADER_SIGNATURE: u32 = 0x0201_4b50;
const END_RECORD_SIGNATURE: u32 = 0x0605_4b50;
const VERSION_NEEDED: u16 = 10;
const VERSION_MADE_BY: u16 = 20;
const DOS_EPOCH_DATE: u16 = 0x0021;

/// An archive and the start of each of its records.
pub struct Built {
    pub bytes: Vec<u8>,
    pub locals: Vec<usize>,
    pub centrals: Vec<usize>,
    pub end: usize,
}

impl Built {
    pub fn set_u16(&mut self, at: usize, value: u16) {
        self.bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }

    pub fn set_u32(&mut self, at: usize, value: u32) {
        self.bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    pub fn get_u32(&self, at: usize) -> u32 {
        let mut word = [0u8; 4];
        word.copy_from_slice(&self.bytes[at..at + 4]);
        u32::from_le_bytes(word)
    }
}

/// Writes `entries` exactly as the server's stored ZIP writer does.
pub fn write(entries: &[(&str, &[u8])]) -> Built {
    let mut archive = Vec::new();
    let mut directory = Vec::new();
    let mut locals = Vec::new();
    let mut central_starts = Vec::new();
    for (name, content) in entries {
        let offset = archive.len();
        let size = content.len() as u32;
        let checksum = super::super::crc::crc32(content);
        locals.push(offset);
        write_local_header(&mut archive, name, size, checksum);
        archive.extend_from_slice(content);
        central_starts.push(directory.len());
        write_central_header(&mut directory, name, size, checksum, offset as u32);
    }
    let directory_offset = archive.len();
    let directory_size = directory.len();
    archive.extend_from_slice(&directory);
    let end = archive.len();
    write_end_record(
        &mut archive,
        entries.len() as u16,
        directory_size as u32,
        directory_offset as u32,
    );
    Built {
        bytes: archive,
        locals,
        centrals: central_starts
            .into_iter()
            .map(|start| start + directory_offset)
            .collect(),
        end,
    }
}

fn write_local_header(out: &mut Vec<u8>, name: &str, size: u32, checksum: u32) {
    push_u32(out, LOCAL_HEADER_SIGNATURE);
    push_u16(out, VERSION_NEEDED);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, DOS_EPOCH_DATE);
    push_u32(out, checksum);
    push_u32(out, size);
    push_u32(out, size);
    push_u16(out, name.len() as u16);
    push_u16(out, 0);
    out.extend_from_slice(name.as_bytes());
}

fn write_central_header(out: &mut Vec<u8>, name: &str, size: u32, checksum: u32, offset: u32) {
    push_u32(out, CENTRAL_HEADER_SIGNATURE);
    push_u16(out, VERSION_MADE_BY);
    push_u16(out, VERSION_NEEDED);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, DOS_EPOCH_DATE);
    push_u32(out, checksum);
    push_u32(out, size);
    push_u32(out, size);
    push_u16(out, name.len() as u16);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u32(out, 0);
    push_u32(out, offset);
    out.extend_from_slice(name.as_bytes());
}

fn write_end_record(out: &mut Vec<u8>, entries: u16, directory_size: u32, directory_offset: u32) {
    push_u32(out, END_RECORD_SIGNATURE);
    push_u16(out, 0);
    push_u16(out, 0);
    push_u16(out, entries);
    push_u16(out, entries);
    push_u32(out, directory_size);
    push_u32(out, directory_offset);
    push_u16(out, 0);
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
