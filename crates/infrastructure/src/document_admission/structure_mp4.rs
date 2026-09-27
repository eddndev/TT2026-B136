use super::{audio_limits, be16, be32, invalid, limit, pixels, require, unsupported, Result};

#[path = "structure_mp4_samples.rs"]
mod samples;

#[derive(Clone, Copy)]
pub(super) struct BoxView {
    pub(super) kind: [u8; 4],
    pub(super) start: usize,
    pub(super) payload: usize,
    pub(super) end: usize,
}
impl BoxView {
    pub(super) fn data(self, bytes: &[u8]) -> &[u8] {
        &bytes[self.payload..self.end]
    }
}
pub(super) fn boxes(bytes: &[u8], start: usize, end: usize) -> Result<Vec<BoxView>> {
    require(start <= end && end <= bytes.len())?;
    let mut found = Vec::new();
    let mut at = start;
    while at < end {
        if found.len() >= 16_384 {
            return Err(limit());
        }
        require(end - at >= 8)?;
        let raw_size = be32(bytes, at)?;
        let (size, header) = if raw_size == 1 {
            require(end - at >= 16)?;
            let value =
                u64::from_be_bytes(bytes[at + 8..at + 16].try_into().map_err(|_| invalid())?);
            (usize::try_from(value).map_err(|_| invalid())?, 16)
        } else if raw_size == 0 {
            (end - at, 8)
        } else {
            (raw_size as usize, 8)
        };
        require(size >= header)?;
        let next = at.checked_add(size).ok_or_else(invalid)?;
        require(next <= end)?;
        found.push(BoxView {
            kind: bytes[at + 4..at + 8].try_into().map_err(|_| invalid())?,
            start: at,
            payload: at + header,
            end: next,
        });
        at = next;
    }
    Ok(found)
}
pub(super) fn unique(values: &[BoxView], kind: &[u8; 4]) -> Result<BoxView> {
    optional(values, kind)?.ok_or_else(invalid)
}
pub(super) fn optional(values: &[BoxView], kind: &[u8; 4]) -> Result<Option<BoxView>> {
    let mut matching = values.iter().filter(|value| &value.kind == kind);
    let value = matching.next().copied();
    require(matching.next().is_none())?;
    Ok(value)
}
fn children(bytes: &[u8], value: BoxView) -> Result<Vec<BoxView>> {
    boxes(bytes, value.payload, value.end)
}
fn bound_tree(bytes: &[u8], values: &[BoxView], depth: usize, budget: &mut usize) -> Result<()> {
    if depth > 16 {
        return Err(limit());
    }
    for value in values {
        *budget = budget.checked_sub(1).ok_or_else(limit)?;
        match &value.kind {
            b"moof" | b"mvex" | b"pssh" | b"sinf" => return Err(unsupported()),
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"dinf" | b"edts" | b"udta" => {
                bound_tree(bytes, &children(bytes, *value)?, depth + 1, budget)?;
            }
            b"meta" => {
                require(value.end - value.payload >= 4)?;
                bound_tree(
                    bytes,
                    &boxes(bytes, value.payload + 4, value.end)?,
                    depth + 1,
                    budget,
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}
fn brand(bytes: &[u8]) -> bool {
    matches!(
        bytes,
        b"isom"
            | b"iso2"
            | b"iso3"
            | b"iso4"
            | b"iso5"
            | b"iso6"
            | b"mp41"
            | b"mp42"
            | b"avc1"
            | b"M4V "
    )
}
fn file_type(bytes: &[u8], value: BoxView) -> Result<()> {
    let data = value.data(bytes);
    require(data.len() >= 8 && (data.len() - 8).is_multiple_of(4))?;
    if !brand(&data[..4]) {
        return Err(unsupported());
    }
    Ok(())
}
fn data_references(bytes: &[u8], minf: &[BoxView]) -> Result<usize> {
    let dinf = children(bytes, unique(minf, b"dinf")?)?;
    let dref = unique(&dinf, b"dref")?;
    let data = dref.data(bytes);
    require(be32(data, 0)? == 0)?;
    let count = be32(data, 4)? as usize;
    require(count != 0)?;
    if count > 8 {
        return Err(limit());
    }
    let entries = boxes(bytes, dref.payload + 8, dref.end)?;
    require(entries.len() == count)?;
    for entry in entries {
        if &entry.kind != b"url " {
            return Err(unsupported());
        }
        let data = entry.data(bytes);
        if be32(data, 0)? != 1 {
            return Err(unsupported());
        }
        require(data.len() == 4)?;
    }
    Ok(count)
}
fn sample_descriptions(bytes: &[u8], stbl: &[BoxView], kind: &[u8], refs: usize) -> Result<usize> {
    let stsd = unique(stbl, b"stsd")?;
    let data = stsd.data(bytes);
    require(be32(data, 0)? == 0)?;
    let count = be32(data, 4)? as usize;
    require(count != 0)?;
    if count > 8 {
        return Err(limit());
    }
    let descriptions = boxes(bytes, stsd.payload + 8, stsd.end)?;
    require(descriptions.len() == count)?;
    for entry in descriptions {
        let data = entry.data(bytes);
        let reference = usize::from(be16(data, 6)?);
        require(reference != 0 && reference <= refs)?;
        let extra = match (&entry.kind, kind) {
            (b"avc1" | b"avc3", b"vide") => {
                require(data.len() >= 78)?;
                pixels(u32::from(be16(data, 24)?), u32::from(be16(data, 26)?))?;
                let extra = boxes(bytes, entry.payload + 78, entry.end)?;
                let configuration = unique(&extra, b"avcC")?.data(bytes);
                require(configuration.len() >= 7 && configuration[0] == 1)?;
                extra
            }
            (b"mp4a", b"soun") => {
                require(data.len() >= 28)?;
                if be16(data, 8)? != 0 {
                    return Err(unsupported());
                }
                audio_limits(u32::from(be16(data, 16)?), be32(data, 24)? >> 16)?;
                let extra = boxes(bytes, entry.payload + 28, entry.end)?;
                let configuration = unique(&extra, b"esds")?.data(bytes);
                require(
                    configuration.len() >= 5
                        && be32(configuration, 0)? == 0
                        && configuration[4] == 3,
                )?;
                extra
            }
            _ => return Err(unsupported()),
        };
        if extra
            .iter()
            .any(|v| matches!(&v.kind, b"sinf" | b"pssh" | b"schm" | b"tenc"))
        {
            return Err(unsupported());
        }
    }
    Ok(count)
}
fn track(bytes: &[u8], value: BoxView, media: &[(usize, usize)]) -> Result<()> {
    let trak = children(bytes, value)?;
    unique(&trak, b"tkhd")?;
    let mdia = children(bytes, unique(&trak, b"mdia")?)?;
    unique(&mdia, b"mdhd")?;
    let handler = unique(&mdia, b"hdlr")?.data(bytes);
    require(handler.len() >= 24)?;
    let kind = &handler[8..12];
    if !matches!(kind, b"vide" | b"soun") {
        return Err(unsupported());
    }
    let minf = children(bytes, unique(&mdia, b"minf")?)?;
    let refs = data_references(bytes, &minf)?;
    let stbl = children(bytes, unique(&minf, b"stbl")?)?;
    let descriptions = sample_descriptions(bytes, &stbl, kind, refs)?;
    samples::validate(bytes, &stbl, descriptions, media)
}
pub(super) fn inspect(bytes: &[u8]) -> Result<()> {
    let top = boxes(bytes, 0, bytes.len())?;
    let ftyp = unique(&top, b"ftyp")?;
    require(ftyp.start == 0)?;
    file_type(bytes, ftyp)?;
    bound_tree(bytes, &top, 0, &mut 16_384)?;
    let moov = children(bytes, unique(&top, b"moov")?)?;
    unique(&moov, b"mvhd")?;
    let media = top
        .iter()
        .filter(|v| &v.kind == b"mdat")
        .map(|v| (v.payload, v.end))
        .collect::<Vec<_>>();
    require(!media.is_empty())?;
    let tracks = moov
        .iter()
        .filter(|v| &v.kind == b"trak")
        .copied()
        .collect::<Vec<_>>();
    require(!tracks.is_empty())?;
    if tracks.len() > 8 {
        return Err(limit());
    }
    for value in tracks {
        track(bytes, value, &media)?;
    }
    Ok(())
}
