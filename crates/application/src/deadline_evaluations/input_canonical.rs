//! Strict DEVI1 encoding of declarations before any profile-dependent decision.
use super::{encoding, DeadlineEvaluationInput};
use crate::{
    deadline_inputs::{encoding::reader::Reader, DeadlineInputError},
    ApplicationError,
};
use std::collections::HashSet;

pub(super) const PREFIX: &[u8; 5] = b"DEVI1";
const MIN_BYTES: usize = 48;
const MAX_BYTES: usize = 98_897;
pub(super) const MAX_CONDITIONS: usize = 16;

/// Preserve declared operands without fabricating a rule or deciding applicability.
pub fn deadline_evaluation_input_bytes(
    input: &DeadlineEvaluationInput,
) -> Result<Vec<u8>, ApplicationError> {
    validate(input)?;
    Ok(encoding::write::encode(input))
}

/// Reject malformed and alternative encodings before returning exact declarations.
pub fn decode_deadline_evaluation_input(
    bytes: &[u8],
) -> Result<DeadlineEvaluationInput, ApplicationError> {
    if !(MIN_BYTES..=MAX_BYTES).contains(&bytes.len()) {
        return Err(invalid("size is outside the DEVI1 bounds"));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(PREFIX.len())? != PREFIX {
        return Err(invalid("prefix differs from DEVI1"));
    }
    let input = encoding::read::decode(&mut reader)
        .map_err(|error| invalid(format!("malformed field: {error}")))?;
    if !reader.finished() || deadline_evaluation_input_bytes(&input)? != bytes {
        return Err(invalid("input is not a canonical DEVI1 representation"));
    }
    Ok(input)
}

fn validate(input: &DeadlineEvaluationInput) -> Result<(), ApplicationError> {
    let conditions = &input.qualification.conditions;
    if conditions.len() > MAX_CONDITIONS {
        return Err(invalid("condition count exceeds sixteen"));
    }
    let mut seen = HashSet::with_capacity(conditions.len());
    if conditions
        .iter()
        .any(|condition| !seen.insert(condition.id))
    {
        return Err(invalid("condition identifiers are duplicated"));
    }
    Ok(())
}

pub(super) fn invalid(error: impl std::fmt::Display) -> ApplicationError {
    DeadlineInputError::Inconsistent(format!("invalid DEVI1 input: {error}")).into()
}
