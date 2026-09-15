use super::{rejected, Result};

/// Canonical URI spelling used only for package identity comparisons.
pub(super) fn part_name(value: &str) -> Result<String> {
    if value.is_empty() || value.starts_with('/') || value.contains('\\') || value.ends_with('/') {
        return rejected();
    }
    let mut output = String::with_capacity(value.len());
    let mut decoded_name = Vec::with_capacity(value.len());
    let special = value.eq_ignore_ascii_case("[Content_Types].xml");
    for segment in value.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." || segment.ends_with('.') {
            return rejected();
        }
        if !output.is_empty() {
            output.push('/');
            decoded_name.push(b'/');
        }
        let mut bytes = segment.bytes();
        while let Some(byte) = bytes.next() {
            if byte == b'%' {
                let high = hex(bytes.next())?;
                let low = hex(bytes.next())?;
                let decoded = high * 16 + low;
                if decoded.is_ascii_alphanumeric()
                    || b"-._~\\/".contains(&decoded)
                    || decoded < 32
                    || decoded == 127
                {
                    return rejected();
                }
                decoded_name.push(decoded);
                output.push('%');
                output.push(char::from(b"0123456789ABCDEF"[high as usize]));
                output.push(char::from(b"0123456789ABCDEF"[low as usize]));
            } else if byte.is_ascii_alphanumeric()
                || (b"-._~!$&'()+,;=@".contains(&byte) || (special && b"[]".contains(&byte)))
            {
                decoded_name.push(byte);
                output.push(char::from(byte.to_ascii_lowercase()));
            } else {
                return rejected();
            }
        }
    }
    std::str::from_utf8(&decoded_name).map_err(|_| super::Error::StageSupportFormatRejected)?;
    Ok(output)
}
fn hex(value: Option<u8>) -> Result<u8> {
    match value {
        Some(b'0'..=b'9') => Ok(value.unwrap() - b'0'),
        Some(b'a'..=b'f') => Ok(value.unwrap() - b'a' + 10),
        Some(b'A'..=b'F') => Ok(value.unwrap() - b'A' + 10),
        _ => rejected(),
    }
}

pub(super) fn target(source: &str, value: &str) -> Result<String> {
    if value.starts_with("//") || value.contains(['?', '#', ':', '\\']) || value.is_empty() {
        return rejected();
    }
    let mut segments: Vec<&str> = if value.starts_with('/') {
        Vec::new()
    } else {
        source
            .rsplit_once('/')
            .map_or(Vec::new(), |(parent, _)| parent.split('/').collect())
    };
    for segment in value.trim_start_matches('/').split('/') {
        match segment {
            "." => {}
            ".." => {
                if segments.pop().is_none() {
                    return rejected();
                }
            }
            "" => return rejected(),
            _ => segments.push(segment),
        }
    }
    part_name(&segments.join("/"))
}
