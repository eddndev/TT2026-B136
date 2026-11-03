//! Bounded JSON bodies and path values of the contract routes. Every body is
//! a named JSON object without unknown members; identifiers are non-nil
//! UUIDs and binary values canonical standard base64.

use crate::{error::ApiError, judicial_calendars::object::Object};
use application::contracts::{CloseOutcome, ContractStage, NewRequest, SubmittedSignature};
use axum::{body::to_bytes, extract::Request};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{crypto::DocumentId, identity::UserId};
use serde::{de::DeserializeOwned, Deserialize};
use uuid::Uuid;

pub(super) const BODY_LIMIT: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Creation {
    operation_id: String,
    document_id: String,
    title: String,
}

impl Creation {
    pub fn values(self) -> Result<(Uuid, DocumentId, String), ApiError> {
        Ok((
            uuid(&self.operation_id)?,
            DocumentId::from_uuid(uuid(&self.document_id)?),
            self.title,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Subject {
    subject_id: String,
    revision: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Requested {
    request_id: String,
    operation_id: String,
    expected_revision: u32,
    version: u32,
    client_email: String,
    firm_signer_id: String,
    ttl_days: u8,
    ceremony_reference: String,
    #[serde(default)]
    subject: Option<Object<Subject>>,
}

impl Requested {
    pub fn values(self) -> Result<NewRequest, ApiError> {
        let subject = match self.subject {
            Some(Object(value)) => Some((uuid(&value.subject_id)?, counter(&value.revision)?)),
            None => None,
        };
        Ok(NewRequest {
            request: uuid(&self.request_id)?,
            operation: uuid(&self.operation_id)?,
            expected_revision: self.expected_revision,
            version: self.version,
            client_email: self.client_email,
            firm_signer: UserId::from_uuid(uuid(&self.firm_signer_id)?),
            ttl_days: self.ttl_days,
            ceremony_reference: self.ceremony_reference,
            subject,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Abandonment {
    operation_id: String,
    expected_revision: u32,
    reason: String,
}

impl Abandonment {
    pub fn values(self) -> Result<(Uuid, u32, String), ApiError> {
        Ok((
            uuid(&self.operation_id)?,
            self.expected_revision,
            self.reason,
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preparation {
    operation_id: String,
}

impl Preparation {
    pub fn operation(self) -> Result<Uuid, ApiError> {
        uuid(&self.operation_id)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Signing {
    operation_id: String,
    expected_revision: u32,
    statement_base64: String,
    signature_base64: String,
}

impl Signing {
    pub fn values(self) -> Result<SubmittedSignature, ApiError> {
        Ok(SubmittedSignature {
            operation: uuid(&self.operation_id)?,
            expected_revision: self.expected_revision,
            statement: decode(&self.statement_base64, 303)?,
            signature: decode(&self.signature_base64, 384)?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Closing {
    operation_id: String,
    expected_revision: u32,
    outcome: String,
    #[serde(default)]
    reason: Option<String>,
}

pub(super) struct CloseValues {
    pub operation: Uuid,
    pub expected_revision: u32,
    pub outcome: CloseOutcome,
    pub reason: Option<String>,
}

impl Closing {
    pub fn values(self) -> Result<CloseValues, ApiError> {
        Ok(CloseValues {
            operation: uuid(&self.operation_id)?,
            expected_revision: self.expected_revision,
            outcome: CloseOutcome::parse(&self.outcome).ok_or_else(invalid)?,
            reason: self.reason,
        })
    }
}

pub(super) fn stage(raw: &str) -> Result<ContractStage, ApiError> {
    ContractStage::parse(raw).ok_or_else(invalid)
}

/// A decimal 64-bit counter without sign, padding or leading zeros.
fn counter(raw: &str) -> Result<u64, ApiError> {
    let value: u64 = raw.parse().map_err(|_| invalid())?;
    if value.to_string() != raw {
        return Err(invalid());
    }
    Ok(value)
}

fn decode(value: &str, length: usize) -> Result<Vec<u8>, ApiError> {
    if value.len() != length.div_ceil(3) * 4 {
        return Err(invalid());
    }
    let bytes = STANDARD.decode(value).map_err(|_| invalid())?;
    if bytes.len() != length || STANDARD.encode(&bytes) != value {
        return Err(invalid());
    }
    Ok(bytes)
}

pub(super) fn uuid(raw: &str) -> Result<Uuid, ApiError> {
    let value = Uuid::parse_str(raw).map_err(|_| invalid())?;
    if value.is_nil() || value.hyphenated().to_string() != raw.to_ascii_lowercase() {
        return Err(invalid());
    }
    Ok(value)
}

pub(super) async fn read<T: DeserializeOwned>(request: Request) -> Result<T, ApiError> {
    crate::request::json::read::<Object<T>>(
        request,
        BODY_LIMIT,
        "contract_body_too_large",
        "contract JSON exceeds 16 KiB",
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
    ApiError::invalid_body("invalid_contract_request", "invalid contract request")
}
