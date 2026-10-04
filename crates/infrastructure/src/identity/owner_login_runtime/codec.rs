use std::{fmt, marker::PhantomData};

use application::identity::certificate_login::StoredCertificateLogin;
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{crypto::Sha256Digest, owner_certificate_login::LoginNonce};
use serde::{
    de::{value::MapAccessDeserializer, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use uuid::Uuid;

use super::{validation, wire::Envelope};

pub(super) const MAX_JSON: usize = 4 * 1024 * 1024;

pub(super) fn encode(value: &StoredCertificateLogin) -> Option<String> {
    validation::capture(value)?;
    let envelope = Envelope {
        version: 1,
        context: (&value.context).into(),
        statement: STANDARD.encode(value.statement.canonical_bytes()),
    };
    let raw = serde_json::to_string(&envelope).ok()?;
    (raw.len() <= MAX_JSON).then_some(raw)
}

pub(super) fn decode(raw: &str) -> Option<StoredCertificateLogin> {
    if raw.len() > MAX_JSON {
        return None;
    }
    let mut parser = serde_json::Deserializer::from_str(raw);
    let value: Envelope = object(&mut parser).ok()?;
    parser.end().ok()?;
    if value.version != 1 {
        return None;
    }
    let context = value.context.into_context()?;
    let bytes = bytes(&value.statement, 182)?;
    let bytes: [u8; 182] = bytes.try_into().ok()?;
    let nonce = LoginNonce::from_bytes(&bytes[134..166]).ok()?;
    let issued = i64::from_be_bytes(bytes[166..174].try_into().ok()?);
    let expires = i64::from_be_bytes(bytes[174..182].try_into().ok()?);
    let statement = validation::statement(&context, nonce, issued, expires)?;
    if statement.canonical_bytes() != bytes {
        return None;
    }
    let capture = StoredCertificateLogin { context, statement };
    validation::capture(&capture)?;
    Some(capture)
}

pub(super) fn bytes(value: &str, maximum: usize) -> Option<Vec<u8>> {
    if value.is_empty() || value.len() > maximum.div_ceil(3) * 4 {
        return None;
    }
    let decoded = STANDARD.decode(value).ok()?;
    if decoded.is_empty() || decoded.len() > maximum || STANDARD.encode(&decoded) != value {
        return None;
    }
    Some(decoded)
}

pub(super) fn digest(value: &str) -> Option<Sha256Digest> {
    if value.len() != 64 {
        return None;
    }
    let digest = Sha256Digest::from_hex(value).ok()?;
    (digest.to_hex() == value).then_some(digest)
}

pub(super) fn uuid(value: &str) -> Option<Uuid> {
    let uuid = Uuid::parse_str(value).ok()?;
    (!uuid.is_nil() && uuid.to_string() == value).then_some(uuid)
}

// Struct deserialization otherwise also accepts positional arrays. Require
// named objects without discarding duplicate fields through a Value conversion.
pub(super) fn object<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Object<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Object<T> {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an object with exact named fields")
        }
        fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<T, M::Error> {
            T::deserialize(MapAccessDeserializer::new(map))
        }
    }
    deserializer.deserialize_map(Object(PhantomData))
}
