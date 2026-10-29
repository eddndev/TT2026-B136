//! RFC 8785 serialization of the admitted JSON subset.
//!
//! Object members are sorted by the UTF-16 code units of their keys, strings
//! escape only the quotation mark, the reverse solidus and control characters
//! below U+0020, no whitespace is emitted, and integers are written in
//! decimal. Only integers in `0..=MAX_CANONICAL_INTEGER` are admitted, the
//! range the evidence manifest allows.

use super::{parse, ErrorKind, JsonError, Limits, Value, MAX_CANONICAL_INTEGER};

/// Serialize `value` in the canonical form of RFC 8785.
///
/// Fails on an integer outside `0..=MAX_CANONICAL_INTEGER` or an object that
/// repeats a key.
pub fn canonical(value: &Value) -> Result<Vec<u8>, JsonError> {
    let mut output = Vec::new();
    write_value(value, &mut output)?;
    Ok(output)
}

/// Parse `bytes` and admit them only when they are already the canonical
/// serialization of the parsed value.
///
/// A byte order mark, whitespace, a trailing newline, reordered members or
/// an escape that RFC 8785 does not write make the bytes differ from their
/// canonical form and fail with [`ErrorKind::NotCanonical`] at the first
/// differing offset; a rejection by [`parse`] or [`canonical`] keeps its
/// own kind.
pub fn parse_canonical(bytes: &[u8], limits: Limits) -> Result<Value, JsonError> {
    let value = parse(bytes, limits)?;
    let expected = canonical(&value)?;
    if expected != bytes {
        let offset = expected
            .iter()
            .zip(bytes)
            .position(|(left, right)| left != right)
            .unwrap_or_else(|| expected.len().min(bytes.len()));
        return Err(JsonError::new(ErrorKind::NotCanonical, offset));
    }
    Ok(value)
}

fn write_value(value: &Value, output: &mut Vec<u8>) -> Result<(), JsonError> {
    match value {
        Value::Object(members) => write_object(members, output),
        Value::Array(items) => {
            output.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_value(item, output)?;
            }
            output.push(b']');
            Ok(())
        }
        Value::String(text) => {
            write_string(text, output);
            Ok(())
        }
        Value::Integer(number) => {
            if !(0..=MAX_CANONICAL_INTEGER).contains(number) {
                return Err(JsonError::new(ErrorKind::IntegerOutOfRange, 0));
            }
            output.extend_from_slice(number.to_string().as_bytes());
            Ok(())
        }
    }
}

fn write_object(members: &[(String, Value)], output: &mut Vec<u8>) -> Result<(), JsonError> {
    let mut sorted: Vec<&(String, Value)> = members.iter().collect();
    sorted.sort_by(|left, right| left.0.encode_utf16().cmp(right.0.encode_utf16()));
    if sorted.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(JsonError::new(ErrorKind::DuplicateKey, 0));
    }
    output.push(b'{');
    for (index, (key, item)) in sorted.into_iter().enumerate() {
        if index > 0 {
            output.push(b',');
        }
        write_string(key, output);
        output.push(b':');
        write_value(item, output)?;
    }
    output.push(b'}');
    Ok(())
}

fn write_string(text: &str, output: &mut Vec<u8>) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push(b'"');
    for byte in text.bytes() {
        match byte {
            b'"' => output.extend_from_slice(b"\\\""),
            b'\\' => output.extend_from_slice(b"\\\\"),
            0x08 => output.extend_from_slice(b"\\b"),
            0x09 => output.extend_from_slice(b"\\t"),
            0x0a => output.extend_from_slice(b"\\n"),
            0x0c => output.extend_from_slice(b"\\f"),
            0x0d => output.extend_from_slice(b"\\r"),
            0x00..=0x1f => {
                output.extend_from_slice(b"\\u00");
                output.push(HEX[usize::from(byte >> 4)]);
                output.push(HEX[usize::from(byte & 0x0f)]);
            }
            // Multi-byte UTF-8 sequences never contain bytes below 0x80, so
            // copying them byte by byte keeps them intact.
            other => output.push(other),
        }
    }
    output.push(b'"');
}
