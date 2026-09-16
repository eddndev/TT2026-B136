use super::{inconsistent, Result};
use application::{
    hearing_results::HearingResultText,
    procedural_facts::{FactLabel, FactText},
};
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
pub(super) fn owned(value: &Value, maximum_bytes: usize) -> Result<String> {
    let raw = string(value)?;
    if raw.len() > maximum_bytes {
        return Err(inconsistent());
    }
    Ok(raw.into())
}
pub(super) fn counter(value: &Value) -> Result<u32> {
    value
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(inconsistent)
}
pub(super) fn byte(value: &Value) -> Result<u8> {
    u8::try_from(counter(value)?).map_err(|_| inconsistent())
}
pub(super) fn integer(value: &Value) -> Result<i32> {
    value
        .as_i64()
        .and_then(|n| i32::try_from(n).ok())
        .ok_or_else(inconsistent)
}
pub(super) fn uuid(value: &Value) -> Result<Uuid> {
    let raw = string(value)?;
    if raw.len() != 36 {
        return Err(inconsistent());
    }
    let result = Uuid::parse_str(raw).map_err(|_| inconsistent())?;
    if result.to_string() != raw {
        return Err(inconsistent());
    }
    Ok(result)
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
pub(super) fn array(value: &Value, maximum: usize) -> Result<&[Value]> {
    let values = value.as_array().ok_or_else(inconsistent)?;
    if values.len() > maximum {
        return Err(inconsistent());
    }
    Ok(values)
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
exact_text!(text, FactText, 1000);
exact_text!(label, FactLabel, 200);
exact_text!(hearing_text, HearingResultText, 1000);
