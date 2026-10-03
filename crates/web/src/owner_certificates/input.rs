use std::{fmt, marker::PhantomData};

use application::identity::owner_certificates::OwnerRegistrationSubmission;
use axum::{body::to_bytes, extract::Request};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::crypto::Signature;
use serde::{
    de::{value::MapAccessDeserializer, DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer,
};

use crate::error::ApiError;

pub(super) const BODY_LIMIT: usize = 32 * 1024;
const CERTIFICATE_LIMIT: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preparation {
    certificate_base64: String,
}

impl Preparation {
    pub(super) fn certificate(self) -> Result<Vec<u8>, ApiError> {
        decode(&self.certificate_base64, 1, CERTIFICATE_LIMIT)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registration {
    statement_base64: String,
    certificate_der_base64: String,
    signature_base64: String,
}

impl Registration {
    pub(super) fn submission(self) -> Result<OwnerRegistrationSubmission, ApiError> {
        Ok(OwnerRegistrationSubmission {
            statement: decode(&self.statement_base64, 150, 150)?,
            certificate_der: decode(&self.certificate_der_base64, 1, CERTIFICATE_LIMIT)?,
            signature: Signature::from_bytes(decode(&self.signature_base64, 384, 384)?)
                .map_err(|_| invalid())?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Withdrawal {
    pub expected_revision: u32,
}

fn decode(value: &str, min: usize, max: usize) -> Result<Vec<u8>, ApiError> {
    if value.is_empty() || value.len() > max.div_ceil(3) * 4 {
        return Err(invalid());
    }
    let bytes = STANDARD.decode(value).map_err(|_| invalid())?;
    if !(min..=max).contains(&bytes.len()) || STANDARD.encode(&bytes) != value {
        return Err(invalid());
    }
    Ok(bytes)
}

// A map-only wrapper prevents Serde structs from accepting positional arrays.
struct Object<T>(T);
impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = Object<T>;
            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }
            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(Object)
            }
        }
        deserializer.deserialize_map(ObjectVisitor::<T>(PhantomData))
    }
}

pub(super) async fn read<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    crate::request::json::read::<Object<T>>(
        request,
        BODY_LIMIT,
        "owner_certificate_body_too_large",
        "owner certificate JSON exceeds 32 KiB",
    )
    .await
    .map(|value| value.0)
}

pub(super) async fn empty(request: Request) -> Result<(), ApiError> {
    let bytes = to_bytes(request.into_body(), 0)
        .await
        .map_err(|_| invalid())?;
    if !bytes.is_empty() {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn invalid() -> ApiError {
    ApiError::invalid_body(
        "owner_certificate_invalid_input",
        "invalid owner certificate request",
    )
}
