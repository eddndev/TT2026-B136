use super::super::*;
use crate::{deadline_inputs::encoding::write::declared_time, ApplicationError};
use domain::crypto::DocumentHasher;

/// OffsetDateTime equality compares instants and ignores the captured offset.
/// Canonical evidence compares both, including nested source and calendar authors.
pub(super) fn same_body(
    hasher: &dyn DocumentHasher,
    previous: &DeadlineDetail,
    next: &DeadlineDetail,
) -> Result<bool, ApplicationError> {
    let mut previous = previous.clone();
    let mut next = next.clone();
    previous.tracking = None;
    next.tracking = None;
    Ok(deadline_capture_bytes(hasher, &previous)? == deadline_capture_bytes(hasher, &next)?)
}

pub(super) fn same_attention(previous: &DeadlineDetail, next: &DeadlineDetail) -> bool {
    attention(&previous.attention) == attention(&next.attention)
}
fn attention(value: &DeadlineAttention) -> Vec<u8> {
    let mut bytes = Vec::new();
    match value {
        DeadlineAttention::Pending => bytes.push(0),
        DeadlineAttention::Recorded {
            occurred_at,
            statement,
            locator,
        } => {
            bytes.push(1);
            declared_time(&mut bytes, *occurred_at);
            evidence::text(&mut bytes, statement.as_str());
            evidence::text(&mut bytes, locator.as_str());
        }
    }
    bytes
}
