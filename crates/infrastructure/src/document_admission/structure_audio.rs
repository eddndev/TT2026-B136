use super::{audio_limits, invalid, limit, require, unsupported, Result};

fn le16(bytes: &[u8], at: usize) -> Result<u16> {
    let part = bytes.get(at..at + 2).ok_or_else(invalid)?;
    Ok(u16::from_le_bytes(part.try_into().map_err(|_| invalid())?))
}
fn le32(bytes: &[u8], at: usize) -> Result<u32> {
    let part = bytes.get(at..at + 4).ok_or_else(invalid)?;
    Ok(u32::from_le_bytes(part.try_into().map_err(|_| invalid())?))
}
fn id3(bytes: &[u8], at: usize) -> Result<usize> {
    let header = bytes.get(at..at + 10).ok_or_else(invalid)?;
    require(matches!(header[3], 2..=4) && header[4] != 255)?;
    let allowed = match header[3] {
        2 => 0xc0,
        3 => 0xe0,
        _ => 0xf0,
    };
    require(header[5] & !allowed == 0 && header[6..10].iter().all(|v| v & 0x80 == 0))?;
    let length = header[6..10]
        .iter()
        .fold(0usize, |n, v| (n << 7) | usize::from(*v));
    let end = at.checked_add(10 + length).ok_or_else(invalid)?;
    require(end <= bytes.len())?;
    if header[3] == 4 && header[5] & 0x10 != 0 {
        let footer = bytes.get(end..end + 10).ok_or_else(invalid)?;
        require(footer.starts_with(b"3DI") && footer[3..] == header[3..])?;
        Ok(end + 10)
    } else {
        Ok(end)
    }
}
pub(super) fn mp3(bytes: &[u8]) -> Result<()> {
    let (mut at, mut frames) = (0usize, 0usize);
    let mut signature = None;
    while bytes.get(at..at + 3) == Some(b"ID3") {
        at = id3(bytes, at)?;
    }
    while at < bytes.len() {
        if bytes.len() - at == 128 && bytes[at..].starts_with(b"TAG") {
            at += 128;
            break;
        }
        let h = bytes.get(at..at + 4).ok_or_else(invalid)?;
        require(h[0] == 0xff && h[1] & 0xe0 == 0xe0)?;
        let version = (h[1] >> 3) & 3;
        require(version != 1)?;
        if (h[1] >> 1) & 3 != 1 {
            return Err(unsupported());
        }
        let bitrate = usize::from(h[2] >> 4);
        if bitrate == 0 {
            return Err(unsupported());
        }
        require(bitrate != 15 && (h[2] >> 2) & 3 != 3 && h[3] & 3 != 2)?;
        let rates = [44100usize, 48000, 32000];
        let rate = rates[usize::from((h[2] >> 2) & 3)]
            / match version {
                3 => 1,
                2 => 2,
                _ => 4,
            };
        let channels = if h[3] >> 6 == 3 { 1 } else { 2 };
        audio_limits(channels, rate as u32)?;
        let current = (version, rate, channels);
        require(signature.is_none_or(|expected| expected == current))?;
        signature = Some(current);
        let table = if version == 3 {
            [
                0usize, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
            ]
        } else {
            [
                0usize, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160,
            ]
        };
        let length = (if version == 3 { 144_000 } else { 72_000 }) * table[bitrate] / rate
            + usize::from((h[2] >> 1) & 1);
        require(length >= 4 + if h[1] & 1 == 0 { 2 } else { 0 })?;
        at = at.checked_add(length).ok_or_else(invalid)?;
        require(at <= bytes.len())?;
        frames += 1;
    }
    require(frames != 0 && at == bytes.len())
}
fn pcm_format(bytes: &[u8]) -> Result<usize> {
    require(bytes.len() >= 16)?;
    let encoding = le16(bytes, 0)?;
    let channels = u32::from(le16(bytes, 2)?);
    let rate = le32(bytes, 4)?;
    audio_limits(channels, rate)?;
    let bits = le16(bytes, 14)?;
    if !matches!(bits, 8 | 16 | 24 | 32) {
        return Err(unsupported());
    }
    match encoding {
        1 => require(bytes.len() == 16 || (bytes.len() == 18 && le16(bytes, 16)? == 0))?,
        0xfffe => {
            require(bytes.len() == 40 && le16(bytes, 16)? == 22)?;
            let valid_bits = le16(bytes, 18)?;
            require(valid_bits != 0 && valid_bits <= bits)?;
            let mask = le32(bytes, 20)?;
            require(mask == 0 || mask.count_ones() == channels)?;
            if &bytes[24..40] != b"\x01\0\0\0\0\0\x10\0\x80\0\0\xaa\0\x38\x9b\x71" {
                return Err(unsupported());
            }
        }
        _ => return Err(unsupported()),
    }
    let align = channels * u32::from(bits) / 8;
    require(u32::from(le16(bytes, 12)?) == align && le32(bytes, 8)? == align * rate)?;
    Ok(align as usize)
}
fn info_list(bytes: &[u8]) -> Result<()> {
    require(bytes.starts_with(b"INFO"))?;
    let mut at = 4usize;
    while at < bytes.len() {
        let len = le32(bytes, at + 4)? as usize;
        at = at.checked_add(8 + len + len % 2).ok_or_else(invalid)?;
        require(at <= bytes.len())?;
    }
    Ok(())
}
pub(super) fn wav(bytes: &[u8]) -> Result<()> {
    require(bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WAVE"))?;
    require(le32(bytes, 4)? as usize == bytes.len() - 8)?;
    let (mut at, mut count, mut data) = (12usize, 0usize, false);
    let mut alignment = None;
    while at < bytes.len() {
        count += 1;
        if count > 65_536 {
            return Err(limit());
        }
        let size = le32(bytes, at + 4)? as usize;
        let end = at.checked_add(8 + size).ok_or_else(invalid)?;
        require(end + size % 2 <= bytes.len())?;
        let payload = &bytes[at + 8..end];
        match &bytes[at..at + 4] {
            b"fmt " => {
                require(alignment.is_none() && !data)?;
                alignment = Some(pcm_format(payload)?);
            }
            b"data" => {
                let align = alignment.ok_or_else(invalid)?;
                require(!data && size != 0 && size.is_multiple_of(align))?;
                data = true;
            }
            b"LIST" => info_list(payload)?,
            _ => {}
        }
        at = end + size % 2;
    }
    require(alignment.is_some() && data)
}
