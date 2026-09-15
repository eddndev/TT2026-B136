use super::fixtures::{self, Part};
use std::io::{Cursor, Read};

pub fn parts(bytes: &[u8]) -> Vec<Part> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let name = file.name().to_string();
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).unwrap();
            Part { name, bytes }
        })
        .collect()
}

/// Writes physical ZIP32 records, allowing duplicates and either descriptor form.
pub fn archive(parts: &[Part], descriptor: Option<bool>, junk: bool) -> Vec<u8> {
    let mut output = Vec::new();
    let mut central = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        let single = fixtures::zip(std::iter::once(part));
        let header = fixtures::signature(&single, b"PK\x01\x02");
        let crc = dword(&single, header + 16);
        let mut size = dword(&single, header + 20);
        let start = 30 + word(&single, 26) as usize + word(&single, 28) as usize;
        let mut compressed = single[start..start + size as usize].to_vec();
        if junk && index == 0 {
            compressed.extend_from_slice(b"extra");
            size += 5;
        }
        let local = output.len() as u32;
        let flags = 0x0800 | if descriptor.is_some() { 8 } else { 0 };
        output.extend_from_slice(b"PK\x03\x04");
        for value in [20, flags, 8, 0, 0] {
            put_word(&mut output, value);
        }
        for value in [crc, size, part.bytes.len() as u32] {
            put_dword(&mut output, if descriptor.is_some() { 0 } else { value });
        }
        put_word(&mut output, part.name.len() as u16);
        put_word(&mut output, 0);
        output.extend_from_slice(part.name.as_bytes());
        output.extend_from_slice(&compressed);
        if let Some(signed) = descriptor {
            if signed {
                output.extend_from_slice(b"PK\x07\x08");
            }
            for value in [crc, size, part.bytes.len() as u32] {
                put_dword(&mut output, value);
            }
        }
        central.extend_from_slice(b"PK\x01\x02");
        for value in [20, 20, flags, 8, 0, 0] {
            put_word(&mut central, value);
        }
        for value in [crc, size, part.bytes.len() as u32] {
            put_dword(&mut central, value);
        }
        for value in [part.name.len() as u16, 0, 0, 0, 0] {
            put_word(&mut central, value);
        }
        put_dword(&mut central, 0);
        put_dword(&mut central, local);
        central.extend_from_slice(part.name.as_bytes());
    }
    let start = output.len();
    output.extend_from_slice(&central);
    output.extend_from_slice(b"PK\x05\x06");
    for value in [0, 0, parts.len() as u16, parts.len() as u16] {
        put_word(&mut output, value);
    }
    put_dword(&mut output, central.len() as u32);
    put_dword(&mut output, start as u32);
    put_word(&mut output, 0);
    output
}
fn word(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}
fn dword(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}
fn put_word(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
fn put_dword(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
