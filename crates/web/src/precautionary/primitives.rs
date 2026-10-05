use crate::error::ApiError;
use domain::{crypto::Sha256Digest, hearings::HearingTime};
use serde::{
    de::{SeqAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, marker::PhantomData};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "invalid_precautionary_hearing",
        "invalid precautionary hearing request",
    )
}
pub(super) fn uuid(value: &str) -> Result<uuid::Uuid, ApiError> {
    let id = uuid::Uuid::parse_str(value).map_err(|_| invalid())?;
    if id.to_string() != value {
        return Err(invalid());
    }
    Ok(id)
}
pub(super) fn digest(value: &str) -> Result<Sha256Digest, ApiError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    Sha256Digest::from_hex(value).map_err(|_| invalid())
}
pub(super) fn number(value: &str) -> Result<u32, ApiError> {
    if value.is_empty()
        || value.len() > 10
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }
    value.parse().map_err(|_| invalid())
}
pub(super) fn empty(query: Option<&str>) -> Result<(), ApiError> {
    if query.is_some_and(|q| !q.is_empty()) {
        Err(invalid())
    } else {
        Ok(())
    }
}
pub(super) fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
pub(super) fn selected<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Vec<T>, D::Error> {
    struct Selection<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Selection<T> {
        type Value = Vec<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("at most 32 selections")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Self::Value, A::Error> {
            let mut values = Vec::new();
            while let Some(value) = a.next_element()? {
                if values.len() == 32 {
                    return Err(serde::de::Error::custom("selection exceeds 32 entries"));
                }
                values.push(value);
            }
            Ok(values)
        }
    }
    d.deserialize_seq(Selection(PhantomData))
}
pub(super) fn scheduled(value: &str) -> Result<HearingTime, ApiError> {
    let b = value.as_bytes();
    let shape = matches!(b.len(), 20 | 25)
        && b.get(4) == Some(&b'-')
        && b.get(7) == Some(&b'-')
        && b.get(10) == Some(&b'T')
        && b.get(13) == Some(&b':')
        && b.get(16) == Some(&b':')
        && [0..4, 5..7, 8..10, 11..13, 14..16, 17..19]
            .into_iter()
            .all(|r| b.get(r).is_some_and(|v| v.iter().all(u8::is_ascii_digit)))
        && ((b.len() == 20 && b[19] == b'Z')
            || (b.len() == 25
                && matches!(b[19], b'+' | b'-')
                && b[22] == b':'
                && b[20..22].iter().all(u8::is_ascii_digit)
                && b[23..25].iter().all(u8::is_ascii_digit)));
    if !shape || value.ends_with("-00:00") || &value[17..19] > "59" {
        return Err(invalid());
    }
    HearingTime::new(OffsetDateTime::parse(value, &Rfc3339).map_err(|_| invalid())?)
        .map_err(|_| invalid())
}
