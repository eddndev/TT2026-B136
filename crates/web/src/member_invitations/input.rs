use std::error::Error;

use application::identity::invitations::{
    InvitationId, InvitationIssueRequest, InvitationQuery, InvitationResendRequest,
};
use axum::{body::to_bytes, extract::Request, http::header::CONTENT_TYPE};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::identity::Role;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::error::InvitationError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Issue {
    operation_id: String,
    email: String,
    role: String,
}
impl Issue {
    pub(super) fn request(self) -> Result<InvitationIssueRequest, InvitationError> {
        let role = match self.role.as_str() {
            "litigator" => Role::Litigator,
            "paralegal" => Role::Paralegal,
            _ => return Err(InvitationError::Invalid),
        };
        Ok(InvitationIssueRequest {
            operation_id: uuid(&self.operation_id)?,
            email: self.email,
            role,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Resend {
    operation_id: String,
    expected_revision: String,
}
impl Resend {
    pub(super) fn request(self) -> Result<InvitationResendRequest, InvitationError> {
        let value = &self.expected_revision;
        if value.is_empty()
            || value.len() > 19
            || value.starts_with('0')
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(InvitationError::Invalid);
        }
        let expected_revision = value.parse::<u64>().map_err(|_| InvitationError::Invalid)?;
        if expected_revision >= i64::MAX as u64 {
            return Err(InvitationError::Invalid);
        }
        Ok(InvitationResendRequest {
            operation_id: uuid(&self.operation_id)?,
            expected_revision,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct List {
    #[serde(default = "default_limit")]
    limit: u16,
    after: Option<String>,
}
fn default_limit() -> u16 {
    50
}
impl List {
    pub(super) fn query(self) -> Result<InvitationQuery, InvitationError> {
        if !(1..=100).contains(&self.limit) {
            return Err(InvitationError::Invalid);
        }
        Ok(InvitationQuery {
            after: self.after.as_deref().map(id).transpose()?,
            limit: self.limit,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recipient {
    #[serde(deserialize_with = "sensitive")]
    token: Zeroizing<String>,
    operation_id: String,
    #[serde(deserialize_with = "sensitive")]
    password: Zeroizing<String>,
}
impl Recipient {
    pub(super) fn material(self) -> Result<Material, InvitationError> {
        material(&self.token, &self.operation_id, self.password)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Finish {
    #[serde(deserialize_with = "sensitive")]
    token: Zeroizing<String>,
    operation_id: String,
    #[serde(deserialize_with = "sensitive")]
    password: Zeroizing<String>,
    #[serde(deserialize_with = "sensitive")]
    code: Zeroizing<String>,
}
impl Finish {
    pub(super) fn material(self) -> Result<(Material, Zeroizing<String>), InvitationError> {
        if self.code.len() != 6 || !self.code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(InvitationError::Rejected);
        }
        Ok((
            material(&self.token, &self.operation_id, self.password)?,
            self.code,
        ))
    }
}

pub(super) struct Material {
    pub token: Zeroizing<[u8; 32]>,
    pub operation: Uuid,
    pub password: Zeroizing<String>,
}
fn material(
    token: &str,
    operation: &str,
    password: Zeroizing<String>,
) -> Result<Material, InvitationError> {
    if !(12..=1024).contains(&password.len()) {
        return Err(InvitationError::Rejected);
    }
    Ok(Material {
        token: decode(token)?,
        operation: uuid(operation)?,
        password,
    })
}
fn sensitive<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}
fn decode(value: &str) -> Result<Zeroizing<[u8; 32]>, InvitationError> {
    if value.len() != 43
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err(InvitationError::Rejected);
    }
    let mut decoded = Zeroizing::new([0; 32]);
    let length = URL_SAFE_NO_PAD
        .decode_slice(value, decoded.as_mut())
        .map_err(|_| InvitationError::Rejected)?;
    if length != 32 || Zeroizing::new(URL_SAFE_NO_PAD.encode(decoded.as_ref())).as_str() != value {
        return Err(InvitationError::Rejected);
    }
    Ok(decoded)
}
fn uuid(value: &str) -> Result<Uuid, InvitationError> {
    let id = value
        .parse::<Uuid>()
        .map_err(|_| InvitationError::Invalid)?;
    if id.is_nil() {
        return Err(InvitationError::Invalid);
    }
    Ok(id)
}
pub(super) fn id(value: &str) -> Result<InvitationId, InvitationError> {
    uuid(value).map(InvitationId::from_uuid)
}
pub(super) fn no_query(request: &Request) -> Result<(), InvitationError> {
    if request.uri().query().is_some() {
        return Err(InvitationError::Invalid);
    }
    Ok(())
}
pub(super) async fn empty(request: Request) -> Result<(), InvitationError> {
    to_bytes(request.into_body(), 0)
        .await
        .map_err(|_| InvitationError::Invalid)?;
    Ok(())
}
pub(super) async fn read<T: DeserializeOwned>(request: Request) -> Result<T, InvitationError> {
    no_query(&request)?;
    let mut types = request.headers().get_all(CONTENT_TYPE).iter();
    let content_type = types.next().and_then(|value| value.to_str().ok());
    if types.next().is_some() || !content_type.is_some_and(is_json) {
        return Err(InvitationError::MediaType);
    }
    let bytes = to_bytes(request.into_body(), 16 * 1024)
        .await
        .map_err(|error| {
            if error
                .source()
                .is_some_and(|cause| cause.is::<http_body_util::LengthLimitError>())
            {
                InvitationError::TooLarge
            } else {
                InvitationError::Json
            }
        })?;
    if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'{') {
        return Err(InvitationError::Json);
    }
    serde_json::from_slice(&bytes).map_err(|_| InvitationError::Json)
}
fn is_json(value: &str) -> bool {
    let mime = value
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    mime == "application/json" || (mime.starts_with("application/") && mime.ends_with("+json"))
}
