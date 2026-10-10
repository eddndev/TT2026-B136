use crate::error::ApiError;
use axum::{body::to_bytes, extract::Request};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    crypto::Signature,
    document_signing::{DocumentConsent, SigningBindingStatement},
};
use serde::{
    de::{value::MapAccessDeserializer, DeserializeOwned, MapAccess, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, marker::PhantomData};
use uuid::Uuid;

pub(super) const BODY_LIMIT: usize = 32 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BindingPreparation {
    certificate_base64: String,
}
impl BindingPreparation {
    pub fn certificate(self) -> Result<Vec<u8>, ApiError> {
        decode(&self.certificate_base64, 1, 16 * 1024)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Registration {
    statement_base64: String,
    certificate_der_base64: String,
    signature_base64: String,
}
pub(super) struct BindingSubmission {
    pub statement: SigningBindingStatement,
    pub certificate: Vec<u8>,
    pub signature: Signature,
}
impl Registration {
    pub fn submission(self) -> Result<BindingSubmission, ApiError> {
        Ok(BindingSubmission {
            statement: SigningBindingStatement::from_canonical_bytes(&decode(
                &self.statement_base64,
                142,
                142,
            )?)
            .map_err(|_| invalid())?,
            certificate: decode(&self.certificate_der_base64, 1, 16 * 1024)?,
            signature: signature(&self.signature_base64)?,
        })
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DocumentPreparation {
    operation_id: String,
    pdf_signature_base64: String,
}
impl DocumentPreparation {
    pub fn values(self) -> Result<(Uuid, Signature), ApiError> {
        Ok((
            uuid(&self.operation_id)?,
            signature(&self.pdf_signature_base64)?,
        ))
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Submission {
    consent_base64: String,
    pdf_signature_base64: String,
    consent_signature_base64: String,
}
pub(super) struct ConsentSubmission {
    pub consent: DocumentConsent,
    pub pdf_signature: Signature,
    pub consent_signature: Signature,
}
impl Submission {
    pub fn values(self) -> Result<ConsentSubmission, ApiError> {
        Ok(ConsentSubmission {
            consent: DocumentConsent::from_canonical_bytes(&decode(
                &self.consent_base64,
                258,
                258,
            )?)
            .map_err(|_| invalid())?,
            pdf_signature: signature(&self.pdf_signature_base64)?,
            consent_signature: signature(&self.consent_signature_base64)?,
        })
    }
}
fn signature(encoded: &str) -> Result<Signature, ApiError> {
    Signature::from_bytes(decode(encoded, 384, 384)?).map_err(|_| invalid())
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
pub(super) fn uuid(raw: &str) -> Result<Uuid, ApiError> {
    let value = Uuid::parse_str(raw).map_err(|_| invalid())?;
    if value.is_nil() {
        return Err(invalid());
    }
    Ok(value)
}
// JSON arrays must not be accepted as positional representations of named fields.
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
        "signing_body_too_large",
        "individual signing JSON exceeds 32 KiB",
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
        "invalid_signing_request",
        "invalid individual signing request",
    )
}
