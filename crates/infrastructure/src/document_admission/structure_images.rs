use super::{be16, be32, invalid, limit, pixels, require, unsupported, Result};

pub(super) fn png(bytes: &[u8]) -> Result<()> {
    require(bytes.starts_with(b"\x89PNG\r\n\x1a\n"))?;
    let (mut at, mut count) = (8usize, 0usize);
    let (mut depth, mut color) = (0u8, 0u8);
    let (mut palette, mut data, mut ended_data) = (false, false, false);
    let mut data_len = 0usize;
    while at < bytes.len() {
        count += 1;
        if count > 65_536 {
            return Err(limit());
        }
        let len = be32(bytes, at)? as usize;
        let end = at
            .checked_add(12)
            .and_then(|n| n.checked_add(len))
            .ok_or_else(invalid)?;
        require(end <= bytes.len())?;
        let kind = &bytes[at + 4..at + 8];
        require(kind.iter().all(u8::is_ascii_alphabetic) && kind[2].is_ascii_uppercase())?;
        let payload = &bytes[at + 8..end - 4];
        require(crc32fast::hash(&bytes[at + 4..end - 4]) == be32(bytes, end - 4)?)?;
        if count == 1 {
            require(kind == b"IHDR")?;
        }
        match kind {
            b"IHDR" => {
                require(count == 1 && len == 13)?;
                pixels(be32(payload, 0)?, be32(payload, 4)?)?;
                depth = payload[8];
                color = payload[9];
                require(match color {
                    0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
                    2 | 4 | 6 => matches!(depth, 8 | 16),
                    3 => matches!(depth, 1 | 2 | 4 | 8),
                    _ => false,
                })?;
                require(payload[10] == 0 && payload[11] == 0 && payload[12] <= 1)?;
            }
            b"PLTE" => {
                require(!palette && !data && !matches!(color, 0 | 4))?;
                require(len != 0 && len <= 768 && len.is_multiple_of(3))?;
                if color == 3 {
                    require(len / 3 <= 1usize << depth)?;
                }
                palette = true;
            }
            b"IDAT" => {
                require(!ended_data && (color != 3 || palette))?;
                data = true;
                data_len += len;
            }
            b"IEND" => {
                require(len == 0 && data && data_len != 0 && end == bytes.len())?;
                return Ok(());
            }
            b"acTL" | b"fcTL" | b"fdAT" => return Err(unsupported()),
            _ if kind[0].is_ascii_uppercase() => return Err(unsupported()),
            _ => {
                if data {
                    ended_data = true;
                }
            }
        }
        at = end;
    }
    Err(invalid())
}

fn quantization(payload: &[u8]) -> Result<()> {
    let mut at = 0;
    while at < payload.len() {
        let info = payload[at];
        require(info & 15 <= 3 && info >> 4 <= 1)?;
        at += 1 + 64 * (usize::from(info >> 4) + 1);
        require(at <= payload.len())?;
    }
    require(at != 0)
}
fn huffman(payload: &[u8]) -> Result<()> {
    let mut at = 0;
    while at < payload.len() {
        require(payload.len() - at >= 17)?;
        require(payload[at] & 15 <= 3 && payload[at] >> 4 <= 1)?;
        let symbols = payload[at + 1..at + 17]
            .iter()
            .map(|v| usize::from(*v))
            .sum::<usize>();
        require(symbols != 0 && symbols <= 256)?;
        at += 17 + symbols;
        require(at <= payload.len())?;
    }
    require(at != 0)
}
pub(super) fn jpeg(bytes: &[u8]) -> Result<()> {
    require(bytes.starts_with(b"\xff\xd8"))?;
    let (mut at, mut components, mut scans) = (2usize, 0usize, 0usize);
    while at < bytes.len() {
        require(bytes[at] == 0xff)?;
        while bytes.get(at) == Some(&0xff) {
            at += 1;
        }
        let kind = *bytes.get(at).ok_or_else(invalid)?;
        at += 1;
        if kind == 0xd9 {
            return require(components != 0 && scans != 0 && at == bytes.len());
        }
        require(!matches!(kind, 0 | 0xd8 | 0xd0..=0xd7))?;
        let size = usize::from(be16(bytes, at)?);
        require(size >= 2)?;
        let end = at.checked_add(size).ok_or_else(invalid)?;
        require(end <= bytes.len())?;
        let payload = &bytes[at + 2..end];
        at = end;
        match kind {
            0xc0..=0xc2 => {
                require(components == 0 && payload.len() >= 6)?;
                require(payload[0] == 8 || (kind != 0xc0 && payload[0] == 12))?;
                pixels(u32::from(be16(payload, 3)?), u32::from(be16(payload, 1)?))?;
                components = usize::from(payload[5]);
                require(matches!(components, 1 | 3 | 4) && payload.len() == 6 + 3 * components)?;
                for index in 0..components {
                    let component = &payload[6 + 3 * index..9 + 3 * index];
                    require(component[1] >> 4 != 0 && component[1] >> 4 <= 4)?;
                    require(component[1] & 15 != 0 && component[1] & 15 <= 4 && component[2] <= 3)?;
                    for previous in 0..index {
                        require(payload[6 + 3 * previous] != component[0])?;
                    }
                }
            }
            0xc4 => huffman(payload)?,
            0xdb => quantization(payload)?,
            0xdd => require(payload.len() == 2)?,
            0xda => {
                require(components != 0 && !payload.is_empty())?;
                let selected = usize::from(payload[0]);
                require(
                    selected != 0 && selected <= components && payload.len() == 4 + 2 * selected,
                )?;
                scans += 1;
                let start = at;
                loop {
                    let value = *bytes.get(at).ok_or_else(invalid)?;
                    if value != 0xff {
                        at += 1;
                        continue;
                    }
                    let marker = at;
                    while bytes.get(at) == Some(&0xff) {
                        at += 1;
                    }
                    let code = *bytes.get(at).ok_or_else(invalid)?;
                    if code == 0 || (0xd0..=0xd7).contains(&code) {
                        at += 1;
                        continue;
                    }
                    require(marker > start)?;
                    at = marker;
                    break;
                }
            }
            0xe0..=0xef | 0xfe => {}
            _ => return Err(unsupported()),
        }
    }
    Err(invalid())
}
