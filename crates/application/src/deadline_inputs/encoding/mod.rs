//! Bounded reversible DINP1 encoding of declared operands and exact selections.
pub(crate) mod read;
pub(crate) mod reader;
pub(crate) mod write;

use super::DeadlineInputRequest;
use crate::ApplicationError;

const PREFIX: &[u8; 5] = b"DINP1";
const MIN_BYTES: usize = 36;
const MAX_BYTES: usize = 8881;

/// Encode the request without deciding whether its selections can produce a calculation.
pub fn deadline_input_request_bytes(request: &DeadlineInputRequest) -> Vec<u8> {
    write::encode(request)
}

/// Reject malformed or noncanonical input before returning validated domain values.
pub fn decode_deadline_input_request(
    bytes: &[u8],
) -> Result<DeadlineInputRequest, ApplicationError> {
    if !(MIN_BYTES..=MAX_BYTES).contains(&bytes.len()) {
        return Err(invalid("request size is outside the DINP1 bounds"));
    }
    let mut reader = reader::Reader::new(bytes);
    if reader.take(PREFIX.len())? != PREFIX {
        return Err(invalid("request prefix differs from DINP1"));
    }
    let request = read::decode(&mut reader)?;
    if !reader.finished() || deadline_input_request_bytes(&request) != bytes {
        return Err(invalid("request is not a canonical DINP1 representation"));
    }
    Ok(request)
}

fn invalid(error: impl std::fmt::Display) -> ApplicationError {
    super::inconsistent(format!("invalid DINP1 request: {error}"))
}
