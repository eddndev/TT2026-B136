use application::identity::certificate_login::StoredCertificateLogin;
use application::ApplicationError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::crypto::DocumentHasher;

use super::{codec, invalid, random_bytes, unavailable, validation, RedisOwnerLoginRuntime};
use crate::RingSha256Hasher;

impl RedisOwnerLoginRuntime {
    pub(super) fn create_capture(
        &self,
        value: &StoredCertificateLogin,
        ttl: u64,
    ) -> Result<String, ApplicationError> {
        let (issued, expires) = validation::capture(value).ok_or_else(invalid)?;
        if !(1..=300).contains(&ttl) || expires.checked_sub(issued) != Some(ttl as i64 * 1000) {
            return Err(invalid());
        }
        let raw = codec::encode(value).ok_or_else(invalid)?;
        let token = URL_SAFE_NO_PAD.encode(random_bytes()?.as_ref());
        let created: bool = redis::Script::new(include_str!("capture.lua"))
            .key(key(&token))
            .arg("create")
            .arg(raw)
            .arg(issued)
            .arg(expires)
            .invoke(&mut self.connection()?)
            .map_err(|_| unavailable())?;
        if !created {
            return Err(invalid());
        }
        Ok(token)
    }

    pub(super) fn take_capture(
        &self,
        token: &str,
    ) -> Result<Option<StoredCertificateLogin>, ApplicationError> {
        let taken: Option<(Vec<u8>, i64, i64)> = redis::Script::new(include_str!("capture.lua"))
            .key(key(token))
            .arg("take")
            .invoke(&mut self.connection()?)
            .map_err(|_| unavailable())?;
        let Some((raw, now, expiration)) = taken else {
            return Ok(None);
        };
        let Ok(raw) = std::str::from_utf8(&raw) else {
            return Ok(None);
        };
        let Some(value) = codec::decode(raw) else {
            return Ok(None);
        };
        let Some((issued, expires)) = validation::capture(&value) else {
            return Ok(None);
        };
        if now < issued || now >= expires || expiration != expires {
            return Ok(None);
        }
        Ok(Some(value))
    }
}

fn key(token: &str) -> String {
    format!(
        "identity:certificate-login:{}",
        RingSha256Hasher.hash_bytes(token.as_bytes()).to_hex()
    )
}
