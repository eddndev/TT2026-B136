use application::identity::certificate_login::{CertificateMfaChallenge, MfaChallenge};
use application::identity::LoginChallengeIdentity;
use application::ApplicationError;

use super::{
    certificate_wire::{self, Challenge},
    wire, RedisSessionStore,
};

impl RedisSessionStore {
    pub(super) fn create_certificate_mfa(
        &self,
        value: &CertificateMfaChallenge,
    ) -> Result<String, ApplicationError> {
        let (from, until, deadline) =
            certificate_wire::challenge_bounds(value).ok_or_else(invalid)?;
        let raw = serde_json::to_string(&Challenge::from_value(value)).map_err(|_| invalid())?;
        if raw.len() > 16_384 {
            return Err(invalid());
        }
        let token = self.random_token()?;
        let created: bool = redis::Script::new(include_str!("challenge.lua"))
            .key(self.digest_key("challenge", token.as_bytes()))
            .arg("create")
            .arg(&raw)
            .arg(from)
            .arg(until)
            .arg(deadline)
            .invoke(&mut self.connection().map_err(|_| unavailable())?)
            .map_err(|_| unavailable())?;
        if !created {
            return Err(invalid());
        }
        Ok(token)
    }

    pub(super) fn take_typed_mfa(
        &self,
        token: &str,
    ) -> Result<Option<MfaChallenge>, ApplicationError> {
        let value: Option<(String, i64, i64)> = redis::Script::new(include_str!("challenge.lua"))
            .key(self.digest_key("challenge", token.as_bytes()))
            .arg("take")
            .invoke(&mut self.connection().map_err(|_| unavailable())?)
            .map_err(|_| unavailable())?;
        let Some((raw, now, expiration)) = value else {
            return Ok(None);
        };
        if let Some(challenge) = wire::parse::<Challenge>(&raw) {
            return Ok(challenge
                .decode(now, expiration)
                .map(|value| MfaChallenge::Certificate(value.into())));
        }
        // Exact bare password objects are the only legacy representation.
        let password = wire::parse::<LoginChallengeIdentity>(&raw)
            .filter(|value| value.auth_generation <= i64::MAX as u64);
        Ok(password.map(MfaChallenge::Password))
    }
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidInput("invalid certificate MFA state".into())
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("certificate MFA store unavailable".into())
}
