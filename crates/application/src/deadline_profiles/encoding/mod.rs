//! Reversible DPRF1 definitions with bounded fields and exact example operands.
mod read;
mod read_expected;
mod read_fields;
mod reader;
mod write;
mod write_expected;
mod write_fields;

use super::{DeadlineProfileDefinition, DeadlineProfileError};

const PREFIX: &[u8; 5] = b"DPRF1";
const MIN_BYTES: usize = 202;
// Conservative sum of bounded fields, not a claim that all maxima coexist.
const MAX_BYTES: usize = 3_261_697;

/// Preserve the definition, ordered corpus and original expected-instant offsets.
pub fn deadline_profile_definition_bytes(profile: &DeadlineProfileDefinition) -> Vec<u8> {
    write::encode(profile)
}

/// Decode bounded fields and reproduce the stated corpus before accepting a definition.
pub fn decode_deadline_profile_definition(
    bytes: &[u8],
) -> Result<DeadlineProfileDefinition, DeadlineProfileError> {
    if !(MIN_BYTES..=MAX_BYTES).contains(&bytes.len()) {
        return Err(invalid("definition length"));
    }
    let mut reader = reader::Reader::new(bytes);
    if reader.take(PREFIX.len())? != PREFIX {
        return Err(invalid("definition prefix"));
    }
    let input = read::decode(&mut reader)?;
    if !reader.finished() {
        return Err(invalid("trailing bytes"));
    }
    let profile = DeadlineProfileDefinition::new(input)?;
    if deadline_profile_definition_bytes(&profile) != bytes {
        return Err(invalid("noncanonical definition"));
    }
    Ok(profile)
}

fn invalid(_error: impl std::fmt::Display) -> DeadlineProfileError {
    DeadlineProfileError::Invalid("DPRF1 encoding")
}
