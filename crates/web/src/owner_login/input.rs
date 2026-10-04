use std::{fmt, marker::PhantomData};

use axum::extract::Request;
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use domain::identity::UserId;
use serde::{
    de::{value::MapAccessDeserializer, DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::error::ApiError;

pub(super) const START_LIMIT: usize = 1024;
pub(super) const PROOF_LIMIT: usize = 2048;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Start {
    owner_id: String,
    binding_id: String,
}

impl Start {
    pub(super) fn identities(self) -> Result<(UserId, Uuid), ApiError> {
        Ok((
            UserId::from_uuid(identity(&self.owner_id)?),
            identity(&self.binding_id)?,
        ))
    }
}

fn identity(raw: &str) -> Result<Uuid, ApiError> {
    let value = Uuid::parse_str(raw).map_err(|_| invalid())?;
    if value.is_nil() || value.to_string() != raw {
        return Err(invalid());
    }
    Ok(value)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Proof {
    challenge_token: String,
    signature_base64: String,
}

impl Proof {
    pub(super) fn material(self) -> Result<(Zeroizing<String>, Vec<u8>), ApiError> {
        let token = Zeroizing::new(self.challenge_token);
        if token.len() != 43 || self.signature_base64.len() != 512 {
            return Err(invalid());
        }
        let decoded = Zeroizing::new(
            URL_SAFE_NO_PAD
                .decode(token.as_bytes())
                .map_err(|_| invalid())?,
        );
        if decoded.len() != 32 || URL_SAFE_NO_PAD.encode(decoded.as_slice()) != token.as_str() {
            return Err(invalid());
        }
        let signature = STANDARD
            .decode(&self.signature_base64)
            .map_err(|_| invalid())?;
        if signature.len() != 384 || STANDARD.encode(&signature) != self.signature_base64 {
            return Err(invalid());
        }
        Ok((token, signature))
    }
}

// Restrict struct deserialization to maps so positional arrays are not accepted.
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

pub(super) async fn read<T: DeserializeOwned>(
    request: Request,
    limit: usize,
) -> Result<T, ApiError> {
    if request.uri().query().is_some() {
        return Err(invalid());
    }
    crate::request::json::read::<Object<T>>(
        request,
        limit,
        "owner_login_body_too_large",
        "owner certificate login JSON exceeds the route limit",
    )
    .await
    .map(|value| value.0)
}

fn invalid() -> ApiError {
    ApiError::invalid_body(
        "owner_login_invalid_input",
        "invalid owner certificate login request",
    )
}
