use super::{paths::part_name, rejected, Error, Result};
use std::collections::HashSet;

#[derive(Debug)]
pub(super) struct Entry {
    pub name: String,
    pub key: String,
    pub size: usize,
    pub compressed: usize,
    pub crc: u32,
    pub flags: u16,
    pub method: u16,
    pub local: usize,
    pub directory: bool,
    pub data: usize,
    pub version: u16,
    pub time: u16,
    pub date: u16,
}

pub(super) fn inspect(bytes: &[u8]) -> Result<Vec<Entry>> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::StageSupportTooLarge);
    }
    if bytes.len() < 22 {
        return rejected();
    }
    let end = (bytes.len().saturating_sub(65_557)..=bytes.len() - 22)
        .rev()
        .find(|&p| {
            bytes.get(p..p + 4) == Some(b"PK\x05\x06")
                && p + 22 + u16_at(bytes, p + 20).unwrap_or(0) as usize == bytes.len()
        })
        .ok_or(Error::StageSupportFormatRejected)?;
    let count = u16_at(bytes, end + 10)? as usize;
    let central_size = u32_at(bytes, end + 12)? as usize;
    let central = u32_at(bytes, end + 16)? as usize;
    if u16_at(bytes, end + 4)? != 0
        || u16_at(bytes, end + 6)? != 0
        || u16_at(bytes, end + 8)? as usize != count
        || count == 0
        || count > 1024
        || central.checked_add(central_size) != Some(end)
    {
        return rejected();
    }
    let mut entries = Vec::with_capacity(count);
    let mut names = HashSet::new();
    let mut offset = central;
    for _ in 0..count {
        if bytes.get(offset..offset + 4) != Some(b"PK\x01\x02") {
            return rejected();
        }
        let version = u16_at(bytes, offset + 6)?;
        let flags = u16_at(bytes, offset + 8)?;
        let method = u16_at(bytes, offset + 10)?;
        if ![10, 20].contains(&version)
            || (method == 8 && version != 20)
            || flags & !0x080e != 0
            || (method != 0 && method != 8)
            || (method == 0 && flags & 6 != 0)
            || u16_at(bytes, offset + 34)? != 0
        {
            return rejected();
        }
        let name_len = u16_at(bytes, offset + 28)? as usize;
        let extra_len = u16_at(bytes, offset + 30)? as usize;
        let comment_len = u16_at(bytes, offset + 32)? as usize;
        let raw = slice(bytes, offset + 46, name_len)?;
        let name = std::str::from_utf8(raw).map_err(|_| Error::StageSupportFormatRejected)?;
        if flags & 0x0800 == 0 && !name.is_ascii() {
            return rejected();
        }
        let directory = name.ends_with('/');
        if u16_at(bytes, offset + 4)? >> 8 == 3 {
            let mode = (u32_at(bytes, offset + 38)? >> 16) & 0o170000;
            if mode != 0 && mode != if directory { 0o040000 } else { 0o100000 } {
                return rejected();
            }
        }
        let key = part_name(name.strip_suffix('/').unwrap_or(name))?;
        if !names.insert(key.clone()) {
            return rejected();
        }
        extras(slice(bytes, offset + 46 + name_len, extra_len)?)?;
        let size = u32_at(bytes, offset + 24)? as usize;
        let compressed = u32_at(bytes, offset + 20)? as usize;
        let crc = u32_at(bytes, offset + 16)?;
        if size == u32::MAX as usize
            || compressed == u32::MAX as usize
            || (method == 0 && size != compressed)
            || (directory && (size != 0 || crc != 0))
        {
            return rejected();
        }
        entries.push(Entry {
            name: name.into(),
            key,
            size,
            compressed,
            crc,
            flags,
            method,
            local: u32_at(bytes, offset + 42)? as usize,
            directory,
            data: 0,
            version,
            time: u16_at(bytes, offset + 12)?,
            date: u16_at(bytes, offset + 14)?,
        });
        offset = offset
            .checked_add(46 + name_len + extra_len + comment_len)
            .ok_or(Error::StageSupportFormatRejected)?;
        if offset > end {
            return rejected();
        }
    }
    if offset != end {
        return rejected();
    }
    let mut ordered: Vec<_> = (0..entries.len()).collect();
    ordered.sort_by_key(|index| entries[*index].local);
    if entries[ordered[0]].local != 0 {
        return rejected();
    }
    for (index, &entry_index) in ordered.iter().enumerate() {
        let next = ordered
            .get(index + 1)
            .map_or(central, |&next| entries[next].local);
        entries[entry_index].data = local(bytes, &entries[entry_index], next)?;
    }
    Ok(entries)
}

fn local(bytes: &[u8], entry: &Entry, next: usize) -> Result<usize> {
    let p = entry.local;
    if p >= next || p > bytes.len().saturating_sub(30) {
        return rejected();
    }
    if bytes.get(p..p + 4) != Some(b"PK\x03\x04")
        || p >= next
        || u16_at(bytes, p + 4)? != entry.version
        || u16_at(bytes, p + 6)? != entry.flags
        || u16_at(bytes, p + 10)? != entry.time
        || u16_at(bytes, p + 12)? != entry.date
        || u16_at(bytes, p + 8)? != entry.method
    {
        return rejected();
    }
    let name_len = u16_at(bytes, p + 26)? as usize;
    let extra_len = u16_at(bytes, p + 28)? as usize;
    if slice(bytes, p + 30, name_len)? != entry.name.as_bytes() {
        return rejected();
    }
    extras(slice(bytes, p + 30 + name_len, extra_len)?)?;
    let data = p
        .checked_add(30 + name_len + extra_len)
        .ok_or(Error::StageSupportFormatRejected)?;
    let data_end = data
        .checked_add(entry.compressed)
        .ok_or(Error::StageSupportFormatRejected)?;
    if data_end > next {
        return rejected();
    }
    let values = [entry.crc, entry.compressed as u32, entry.size as u32];
    for (i, value) in values.iter().enumerate() {
        let local_value = u32_at(bytes, p + 14 + i * 4)?;
        if local_value != *value && (entry.flags & 8 == 0 || local_value != 0) {
            return rejected();
        }
    }
    let length = next - data_end;
    if entry.flags & 8 == 0 {
        return if length == 0 { Ok(data) } else { rejected() };
    }
    let descriptor = match length {
        12 => data_end,
        16 if bytes.get(data_end..data_end + 4) == Some(b"PK\x07\x08") => data_end + 4,
        _ => return rejected(),
    };
    for (i, value) in values.iter().enumerate() {
        if u32_at(bytes, descriptor + 4 * i)? != *value {
            return rejected();
        }
    }
    Ok(data)
}
fn extras(bytes: &[u8]) -> Result<()> {
    let mut offset = 0;
    let mut kinds = HashSet::new();
    while offset < bytes.len() {
        let kind = u16_at(bytes, offset)?;
        let len = u16_at(bytes, offset + 2)? as usize;
        if [0x0001, 0x0017, 0x9901, 0x7075].contains(&kind) || !kinds.insert(kind) {
            return rejected();
        }
        slice(bytes, offset + 4, len)?;
        offset += 4 + len;
    }
    Ok(())
}
fn slice(bytes: &[u8], offset: usize, len: usize) -> Result<&[u8]> {
    bytes
        .get(
            offset
                ..offset
                    .checked_add(len)
                    .ok_or(Error::StageSupportFormatRejected)?,
        )
        .ok_or(Error::StageSupportFormatRejected)
}
fn u16_at(bytes: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(slice(bytes, p, 2)?.try_into().unwrap()))
}
fn u32_at(bytes: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(slice(bytes, p, 4)?.try_into().unwrap()))
}
