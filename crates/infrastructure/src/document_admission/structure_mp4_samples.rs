use super::{be32, invalid, optional, require, unique, unsupported, BoxView, Result};

fn table(data: &[u8], stride: usize) -> Result<usize> {
    require(be32(data, 0)? == 0)?;
    let count = be32(data, 4)? as usize;
    require(count != 0)?;
    let expected = count
        .checked_mul(stride)
        .and_then(|n| n.checked_add(8))
        .ok_or_else(invalid)?;
    require(data.len() == expected)?;
    Ok(count)
}
fn sizes(bytes: &[u8], values: &[BoxView]) -> Result<Vec<u32>> {
    if optional(values, b"stz2")?.is_some() {
        return Err(unsupported());
    }
    let data = unique(values, b"stsz")?.data(bytes);
    require(be32(data, 0)? == 0)?;
    let size = be32(data, 4)?;
    let count = be32(data, 8)? as usize;
    require(count != 0 && count <= bytes.len())?;
    if size != 0 {
        require(data.len() == 12)?;
        let total = u64::from(size) * count as u64;
        require(total <= bytes.len() as u64)?;
        Ok(vec![size; count])
    } else {
        require(data.len() == 12 + count.checked_mul(4).ok_or_else(invalid)?)?;
        let mut result = Vec::with_capacity(count);
        for index in 0..count {
            let size = be32(data, 12 + index * 4)?;
            require(size != 0)?;
            result.push(size);
        }
        Ok(result)
    }
}
fn chunks(bytes: &[u8], values: &[BoxView]) -> Result<Vec<u64>> {
    let small = optional(values, b"stco")?;
    let large = optional(values, b"co64")?;
    let (value, stride) = match (small, large) {
        (Some(v), None) => (v, 4),
        (None, Some(v)) => (v, 8),
        _ => return Err(invalid()),
    };
    let data = value.data(bytes);
    let count = table(data, stride)?;
    let mut result = Vec::with_capacity(count);
    for index in 0..count {
        let at = 8 + index * stride;
        result.push(if stride == 4 {
            u64::from(be32(data, at)?)
        } else {
            u64::from_be_bytes(data[at..at + 8].try_into().map_err(|_| invalid())?)
        });
    }
    Ok(result)
}
fn timing(bytes: &[u8], values: &[BoxView], sample_count: usize) -> Result<()> {
    let data = unique(values, b"stts")?.data(bytes);
    let count = table(data, 8)?;
    let mut total = 0u64;
    for index in 0..count {
        let samples = be32(data, 8 + index * 8)?;
        require(samples != 0 && be32(data, 12 + index * 8)? != 0)?;
        total = total.checked_add(u64::from(samples)).ok_or_else(invalid)?;
    }
    require(total == sample_count as u64)
}
pub(super) fn validate(
    bytes: &[u8],
    values: &[BoxView],
    descriptions: usize,
    media: &[(usize, usize)],
) -> Result<()> {
    let sizes = sizes(bytes, values)?;
    timing(bytes, values, sizes.len())?;
    let offsets = chunks(bytes, values)?;
    require(offsets.len() <= sizes.len())?;
    let data = unique(values, b"stsc")?.data(bytes);
    let count = table(data, 12)?;
    require(count <= offsets.len())?;
    let mut mapping = Vec::with_capacity(count);
    for index in 0..count {
        let first = be32(data, 8 + index * 12)? as usize;
        let samples = be32(data, 12 + index * 12)? as usize;
        let description = be32(data, 16 + index * 12)? as usize;
        require(first != 0 && first <= offsets.len() && samples != 0)?;
        require(description != 0 && description <= descriptions)?;
        if index == 0 {
            require(first == 1)?;
        }
        if let Some((previous, _)) = mapping.last() {
            require(first > *previous)?;
        }
        mapping.push((first, samples));
    }
    let (mut map, mut sample) = (0usize, 0usize);
    for (index, offset) in offsets.into_iter().enumerate() {
        while map + 1 < mapping.len() && mapping[map + 1].0 <= index + 1 {
            map += 1;
        }
        let end_sample = sample.checked_add(mapping[map].1).ok_or_else(invalid)?;
        require(end_sample <= sizes.len())?;
        let size = sizes[sample..end_sample]
            .iter()
            .try_fold(0u64, |sum, value| {
                sum.checked_add(u64::from(*value)).ok_or_else(invalid)
            })?;
        let end = offset.checked_add(size).ok_or_else(invalid)?;
        require(
            media
                .iter()
                .any(|(start, stop)| offset >= *start as u64 && end <= *stop as u64),
        )?;
        sample = end_sample;
    }
    require(sample == sizes.len())
}
