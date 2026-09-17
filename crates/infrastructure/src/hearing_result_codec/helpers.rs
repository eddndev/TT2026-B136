use super::{inconsistent, Result};
use application::hearing_results::*;
use domain::crypto::Sha256Digest;
use serde_json::Value;
use uuid::Uuid;

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
pub(super) fn counter(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(inconsistent)
}
pub(super) fn uuid(value: &Value) -> Result<Uuid> {
    let raw = string(value)?;
    if raw.len() != 36 {
        return Err(inconsistent());
    }
    let id = Uuid::parse_str(raw).map_err(|_| inconsistent())?;
    if id.to_string() != raw {
        return Err(inconsistent());
    }
    Ok(id)
}
pub(super) fn digest(value: &Value) -> Result<Sha256Digest> {
    let raw = string(value)?;
    if raw.len() != 64 {
        return Err(inconsistent());
    }
    let result = Sha256Digest::from_hex(raw).map_err(|_| inconsistent())?;
    if result.to_hex() != raw {
        return Err(inconsistent());
    }
    Ok(result)
}
pub(super) fn optional<T>(
    value: &Value,
    parse: impl FnOnce(&Value) -> Result<T>,
) -> Result<Option<T>> {
    if value.is_null() {
        Ok(None)
    } else {
        parse(value).map(Some)
    }
}
macro_rules! exact_text {
    ($function:ident,$kind:ident,$maximum:expr) => {
        pub(super) fn $function(value: &Value) -> Result<$kind> {
            let raw = string(value)?;
            if raw.len() > $maximum * 4 {
                return Err(inconsistent());
            }
            let result = $kind::new(raw).map_err(|_| inconsistent())?;
            if result.as_str() != raw {
                return Err(inconsistent());
            }
            Ok(result)
        }
    };
}
exact_text!(text, HearingResultText, 1000);
exact_text!(capacity, HearingResultCapacity, 100);
exact_text!(observation, HearingResultObservation, 500);
exact_text!(reference, HearingResultReference, 200);
