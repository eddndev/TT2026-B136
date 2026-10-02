use std::error::Error;

use axum::{body::to_bytes, extract::Request, http::header::CONTENT_TYPE};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use zeroize::Zeroizing;

use super::error::ResetError;

const MAX_BODY_BYTES: usize = 16 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestInput {
    #[serde(deserialize_with = "sensitive")]
    pub email: Zeroizing<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CompletionInput {
    #[serde(deserialize_with = "sensitive")]
    pub token: Zeroizing<String>,
    #[serde(deserialize_with = "sensitive")]
    pub new_password: Zeroizing<String>,
}

fn sensitive<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}

pub(super) async fn read<T: DeserializeOwned>(request: Request) -> Result<T, ResetError> {
    if request.uri().query().is_some() {
        return Err(ResetError::Query);
    }
    let mut types = request.headers().get_all(CONTENT_TYPE).iter();
    let content_type = types.next().and_then(|value| value.to_str().ok());
    if types.next().is_some() || !content_type.is_some_and(is_json) {
        return Err(ResetError::MediaType);
    }
    let bytes = to_bytes(request.into_body(), MAX_BODY_BYTES)
        .await
        .map_err(|error| {
            if error
                .source()
                .is_some_and(|cause| cause.is::<http_body_util::LengthLimitError>())
            {
                ResetError::TooLarge
            } else {
                ResetError::Json
            }
        })?;
    // Serde structs can accept positional arrays; this boundary requires an object.
    if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'{') {
        return Err(ResetError::Json);
    }
    serde_json::from_slice(&bytes).map_err(|_| ResetError::Json)
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

pub(super) fn token(value: &str) -> Result<Zeroizing<[u8; 32]>, ResetError> {
    if value.len() != 43
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
    {
        return Err(ResetError::Rejected);
    }
    let mut decoded = Zeroizing::new([0; 32]);
    let length = URL_SAFE_NO_PAD
        .decode_slice(value, decoded.as_mut())
        .map_err(|_| ResetError::Rejected)?;
    if length != 32 {
        return Err(ResetError::Rejected);
    }
    let canonical = Zeroizing::new(URL_SAFE_NO_PAD.encode(&decoded[..]));
    if canonical.as_str() != value {
        return Err(ResetError::Rejected);
    }
    Ok(decoded)
}
