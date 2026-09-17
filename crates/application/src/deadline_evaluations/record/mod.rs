//! Immutable historical output; decoding validates storage shape without running an evaluator.
mod days;
mod model;
mod primitives;
mod read;
mod read_blocks;
mod read_days;
mod validation;
mod write;
mod write_blocks;
mod write_days;

use super::ProfiledDeadlineEvaluation;
use crate::{deadline_inputs::encoding::reader::Reader, ApplicationError};
pub use days::{DeadlineCalendarDayRecord, DeadlineDayCountRecord, DeadlineDayStepRecord};
pub use model::{DeadlineArithmeticRecord, DeadlineEvaluationRecord, DeadlineTraceRecord};

/// Conservative bound: two traces of 1097 days, each with 16 UUID references and
/// at most 1024 UTF-8 explanation bytes, plus scalar output and 20 blocks.
pub const MAX_DEADLINE_EVALUATION_RECORD_BYTES: usize = 3_000_000;

pub fn deadline_evaluation_record_bytes(value: &DeadlineEvaluationRecord) -> Vec<u8> {
    write::encode(value)
}
pub fn decode_deadline_evaluation_record(
    bytes: &[u8],
) -> Result<DeadlineEvaluationRecord, ApplicationError> {
    if bytes.len() > MAX_DEADLINE_EVALUATION_RECORD_BYTES {
        return Err(invalid("record exceeds byte bound"));
    }
    let mut reader = Reader::new(bytes);
    if reader.take(5)? != b"DRES1" {
        return Err(invalid("unknown result prefix"));
    }
    let value = read::decode(&mut reader).map_err(invalid)?;
    if !reader.finished() || deadline_evaluation_record_bytes(&value) != bytes {
        return Err(invalid("noncanonical result"));
    }
    validation::validate(&value)?;
    Ok(value)
}
fn invalid(error: impl std::fmt::Display) -> ApplicationError {
    ApplicationError::InvalidInput(format!("invalid DRES1 result: {error}"))
}
