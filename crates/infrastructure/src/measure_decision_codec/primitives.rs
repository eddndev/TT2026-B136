use super::Result;
use application::{precautionary_measures::MeasureDecisionError, ApplicationError};
use domain::{crypto::Sha256Digest, hearings::HearingNote};
use serde_json::Value;
use uuid::Uuid;

pub(super) fn inconsistent() -> ApplicationError {
    MeasureDecisionError::StoredInconsistent("stored decision values are inconsistent".into())
        .into()
}
pub(super) fn fields(value: &Value, expected: &[&str]) -> Result<()> {
    let object = value.as_object().ok_or_else(inconsistent)?;
    if object.len() != expected.len() || expected.iter().any(|key| !object.contains_key(*key)) {
        return Err(inconsistent());
    }
    Ok(())
}
pub(super) fn string(value: &Value) -> Result<&str> {
    value.as_str().ok_or_else(inconsistent)
}
pub(super) fn integer(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(inconsistent)
}
pub(super) fn uuid(value: &Value) -> Result<Uuid> {
    let raw = string(value)?;
    if raw.len() != 36 {
        return Err(inconsistent());
    }
    let parsed = Uuid::parse_str(raw).map_err(|_| inconsistent())?;
    if parsed.to_string() != raw {
        return Err(inconsistent());
    }
    Ok(parsed)
}
pub(super) fn digest(value: &Value) -> Result<Sha256Digest> {
    let raw = string(value)?;
    if raw.len() != 64 {
        return Err(inconsistent());
    }
    let parsed = Sha256Digest::from_hex(raw).map_err(|_| inconsistent())?;
    if parsed.to_hex() != raw {
        return Err(inconsistent());
    }
    Ok(parsed)
}
pub(super) fn note(value: &Value) -> Result<HearingNote> {
    let raw = string(value)?;
    if raw.len() > 4000 {
        return Err(inconsistent());
    }
    let parsed = HearingNote::new(raw).map_err(|_| inconsistent())?;
    if parsed.as_str() != raw {
        return Err(inconsistent());
    }
    Ok(parsed)
}
pub(super) fn frame(bytes: &[u8], prefix: &[u8], minimum: usize, maximum: usize) -> Result<()> {
    if !(minimum..=maximum).contains(&bytes.len()) || !bytes.starts_with(prefix) {
        return Err(inconsistent());
    }
    Ok(())
}
