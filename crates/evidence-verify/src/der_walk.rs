//! Bounded strict DER reader that locates exact byte ranges.
//!
//! Signatures over CMS signed attributes and encapsulated content are checked
//! over the bytes exactly as they appear in the evidence, never over a
//! re-encoding, so callers need the precise slice of each element. This reader
//! returns borrowed slices of the input and admits only the subset of DER the
//! evidence uses: single-byte identifier octets and definite lengths in their
//! minimal form with at most four length octets. Anything else is rejected
//! rather than repaired. Rationale:
//! `docs/adr/0085-in-browser-evidence-validation.md`.

use core::fmt;

/// Nesting limit callers pass to [`walk`] for evidence structures.
pub const MAX_DEPTH: usize = 32;

/// Bit of the identifier octet that marks a constructed encoding.
const CONSTRUCTED: u8 = 0x20;
/// Low five identifier bits that announce a multi-byte tag number.
const HIGH_TAG_NUMBER: u8 = 0x1F;
/// Length octet that announces the indefinite form.
const INDEFINITE: u8 = 0x80;
/// Most length octets admitted in the long form.
const MAX_LENGTH_OCTETS: usize = 4;

/// Why an input is not admitted as strict DER.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerError {
    /// The input ends before the identifier, the length or the declared value
    /// is complete. A long-form length with more than four length octets is
    /// also reported here: it declares at least 2^32 bytes, more than any
    /// input this reader accepts.
    Truncated,
    /// The length uses the indefinite form, which DER forbids.
    IndefiniteLength,
    /// The length is not in its shortest form: long form for a value under
    /// 128 bytes, or a leading zero length octet.
    NonMinimalLength,
    /// The identifier announces a tag number that needs more than one octet.
    HighTagNumber,
    /// A constructed value holds more elements than the caller admits.
    TooManyChildren,
    /// Bytes follow the single element the caller expected.
    TrailingData,
    /// Constructed elements nest deeper than the caller admits.
    TooDeep,
}

impl DerError {
    /// Stable machine-readable identifier of the error.
    pub fn code(self) -> &'static str {
        match self {
            Self::Truncated => "der.truncated",
            Self::IndefiniteLength => "der.indefinite_length",
            Self::NonMinimalLength => "der.non_minimal_length",
            Self::HighTagNumber => "der.high_tag_number",
            Self::TooManyChildren => "der.too_many_children",
            Self::TrailingData => "der.trailing_data",
            Self::TooDeep => "der.too_deep",
        }
    }
}

impl fmt::Display for DerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Truncated => "DER input is truncated",
            Self::IndefiniteLength => "DER input uses an indefinite length",
            Self::NonMinimalLength => "DER length is not minimally encoded",
            Self::HighTagNumber => "DER tag number needs more than one octet",
            Self::TooManyChildren => "DER value has too many elements",
            Self::TrailingData => "DER element is followed by trailing data",
            Self::TooDeep => "DER elements nest too deeply",
        };
        f.write_str(message)
    }
}

/// One element as it appears in the input.
///
/// Every slice borrows from the input: `raw` is `header` followed by `value`,
/// and `tag` is the first octet of `header`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tlv<'a> {
    /// Identifier octet, including class and constructed bits.
    pub tag: u8,
    /// Identifier and length octets.
    pub header: &'a [u8],
    /// Content octets.
    pub value: &'a [u8],
    /// Identifier, length and content octets.
    pub raw: &'a [u8],
}

impl Tlv<'_> {
    /// Whether the identifier marks a constructed encoding, so the value is
    /// itself a sequence of elements.
    pub fn is_constructed(&self) -> bool {
        self.tag & CONSTRUCTED != 0
    }
}

/// Read the first element of `input` and return it with the bytes after it.
pub fn read(input: &[u8]) -> Result<(Tlv<'_>, &[u8]), DerError> {
    let (&tag, after_tag) = input.split_first().ok_or(DerError::Truncated)?;
    if tag & HIGH_TAG_NUMBER == HIGH_TAG_NUMBER {
        return Err(DerError::HighTagNumber);
    }
    let (length, length_octets) = read_length(after_tag)?;
    let header_len = 1 + length_octets;
    let available = input.len() - header_len;
    if length > available {
        return Err(DerError::Truncated);
    }
    let (raw, rest) = input.split_at(header_len + length);
    let (header, value) = raw.split_at(header_len);
    Ok((
        Tlv {
            tag,
            header,
            value,
            raw,
        },
        rest,
    ))
}

/// Read exactly one element that spans all of `input`.
pub fn read_exact(input: &[u8]) -> Result<Tlv<'_>, DerError> {
    let (tlv, rest) = read(input)?;
    if !rest.is_empty() {
        return Err(DerError::TrailingData);
    }
    Ok(tlv)
}

/// Split the value of a constructed element into its elements.
///
/// At most `max` elements are admitted; one more is [`DerError::TooManyChildren`].
/// An empty value yields no elements for any `max`. Elements are not
/// descended into; use [`walk`] to check nested structure.
pub fn children(value: &[u8], max: usize) -> Result<Vec<Tlv<'_>>, DerError> {
    let mut out = Vec::new();
    let mut rest = value;
    while !rest.is_empty() {
        if out.len() == max {
            return Err(DerError::TooManyChildren);
        }
        let (tlv, after) = read(rest)?;
        out.push(tlv);
        rest = after;
    }
    Ok(out)
}

/// Read exactly one element spanning `input` and check that every constructed
/// element inside it is itself a complete sequence of strict elements.
///
/// The outer element is at depth one and each constructed level adds one; an
/// element deeper than `max_depth` is [`DerError::TooDeep`], so a `max_depth`
/// of zero admits nothing. Primitive values, such as the content of an OCTET
/// STRING, are not interpreted. Recursion is bounded by `max_depth`.
pub fn walk(input: &[u8], max_depth: usize) -> Result<Tlv<'_>, DerError> {
    let tlv = read_exact(input)?;
    check_nested(&tlv, 1, max_depth)?;
    Ok(tlv)
}

fn check_nested(tlv: &Tlv<'_>, depth: usize, max_depth: usize) -> Result<(), DerError> {
    if depth > max_depth {
        return Err(DerError::TooDeep);
    }
    if !tlv.is_constructed() {
        return Ok(());
    }
    let mut rest = tlv.value;
    while !rest.is_empty() {
        let (child, after) = read(rest)?;
        check_nested(&child, depth + 1, max_depth)?;
        rest = after;
    }
    Ok(())
}

/// Decode a definite minimal length, returning it with the number of length
/// octets consumed.
fn read_length(input: &[u8]) -> Result<(usize, usize), DerError> {
    let (&first, after) = input.split_first().ok_or(DerError::Truncated)?;
    if first < INDEFINITE {
        return Ok((usize::from(first), 1));
    }
    if first == INDEFINITE {
        return Err(DerError::IndefiniteLength);
    }
    let count = usize::from(first & !INDEFINITE);
    if count > MAX_LENGTH_OCTETS {
        return Err(DerError::Truncated);
    }
    let octets = after.get(..count).ok_or(DerError::Truncated)?;
    if octets[0] == 0 {
        return Err(DerError::NonMinimalLength);
    }
    let length = octets
        .iter()
        .fold(0u32, |acc, &octet| (acc << 8) | u32::from(octet));
    if length < u32::from(INDEFINITE) {
        return Err(DerError::NonMinimalLength);
    }
    let length = usize::try_from(length).map_err(|_| DerError::Truncated)?;
    Ok((length, 1 + count))
}

#[cfg(test)]
mod tests;
