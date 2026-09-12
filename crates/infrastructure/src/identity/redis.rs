//! Redis-backed challenges, revocable sessions, and login limits.

use std::time::Duration;

use application::identity::{Principal, SessionStore};
use application::ApplicationError;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use domain::crypto::DocumentHasher;
use domain::identity::UserId;
use redis::Commands;

use crate::RingSha256Hasher;

const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(5);

/// Ephemeral identity state stored in Redis under SHA-256-derived keys.
pub struct RedisSessionStore {
    client: redis::Client,
    hasher: RingSha256Hasher,
    connect_timeout: Duration,
    io_timeout: Duration,
}

impl RedisSessionStore {
    pub fn connect(redis_url: &str) -> Result<Self, ApplicationError> {
        Self::connect_with_timeouts(redis_url, DEFAULT_CONNECT_TIMEOUT, DEFAULT_IO_TIMEOUT)
    }

    /// Configures TCP connection and established-socket I/O timeouts.
    ///
    /// The synchronous Redis driver completes authentication, database
    /// selection, and client identification before socket I/O deadlines can
    /// be set. Those handshake reads are not bounded by these settings.
    pub fn connect_with_timeouts(
        redis_url: &str,
        connect_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Self, ApplicationError> {
        if connect_timeout.is_zero() || io_timeout.is_zero() {
            return Err(ApplicationError::InvalidConfiguration(
                "Redis timeouts must be positive".into(),
            ));
        }
        Ok(Self {
            client: redis::Client::open(redis_url).map_err(port_error)?,
            hasher: RingSha256Hasher::new(),
            connect_timeout,
            io_timeout,
        })
    }

    fn connection(&self) -> Result<redis::Connection, ApplicationError> {
        let connection = self
            .client
            .get_connection_with_timeout(self.connect_timeout)
            .map_err(port_error)?;
        connection
            .set_read_timeout(Some(self.io_timeout))
            .map_err(port_error)?;
        connection
            .set_write_timeout(Some(self.io_timeout))
            .map_err(port_error)?;
        Ok(connection)
    }

    fn digest_key(&self, namespace: &str, value: &[u8]) -> String {
        format!(
            "identity:{namespace}:{}",
            self.hasher.hash_bytes(value).to_hex()
        )
    }

    fn random_token(&self) -> Result<String, ApplicationError> {
        let mut bytes = [0u8; 32];
        getrandom::getrandom(&mut bytes).map_err(|error| {
            ApplicationError::Port(format!("session token generation failed: {error}"))
        })?;
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }
}

impl SessionStore for RedisSessionStore {
    fn create_challenge(&self, user_id: UserId, ttl: u64) -> Result<String, ApplicationError> {
        let token = self.random_token()?;
        let key = self.digest_key("challenge", token.as_bytes());
        self.connection()?
            .set_ex::<_, _, ()>(key, user_id.to_string(), ttl)
            .map_err(port_error)?;
        Ok(token)
    }

    fn take_challenge(&self, token: &str) -> Result<Option<UserId>, ApplicationError> {
        let key = self.digest_key("challenge", token.as_bytes());
        let value: Option<String> = redis::cmd("GETDEL")
            .arg(key)
            .query(&mut self.connection()?)
            .map_err(port_error)?;
        value
            .map(|raw| {
                uuid::Uuid::parse_str(&raw)
                    .map(UserId::from_uuid)
                    .map_err(|error| {
                        ApplicationError::Port(format!("redis challenge user is invalid: {error}"))
                    })
            })
            .transpose()
    }

    fn create_session(&self, principal: &Principal, ttl: u64) -> Result<String, ApplicationError> {
        let token = self.random_token()?;
        let key = self.digest_key("session", token.as_bytes());
        let value = serde_json::to_string(principal).map_err(|error| {
            ApplicationError::Port(format!("session serialization failed: {error}"))
        })?;
        self.connection()?
            .set_ex::<_, _, ()>(key, value, ttl)
            .map_err(port_error)?;
        Ok(token)
    }

    fn find_session(&self, token: &str) -> Result<Option<Principal>, ApplicationError> {
        let key = self.digest_key("session", token.as_bytes());
        let value: Option<String> = self.connection()?.get(key).map_err(port_error)?;
        value
            .map(|json| {
                serde_json::from_str(&json).map_err(|error| {
                    ApplicationError::Port(format!("stored session is invalid: {error}"))
                })
            })
            .transpose()
    }

    fn revoke_session(&self, token: &str) -> Result<(), ApplicationError> {
        let key = self.digest_key("session", token.as_bytes());
        self.connection()?.del::<_, ()>(key).map_err(port_error)
    }

    fn failed_password_attempts(&self, email: &str, ttl: u64) -> Result<u32, ApplicationError> {
        let ttl = failure_window(ttl)?;
        let key = self.digest_key("password-failures", email.as_bytes());
        redis::Script::new(
            "local count = redis.call('GET', KEYS[1])
             if not count then return 0 end
             if redis.call('TTL', KEYS[1]) == -1 then
                 redis.call('EXPIRE', KEYS[1], ARGV[1])
             end
             return count",
        )
        .key(key)
        .arg(ttl)
        .invoke(&mut self.connection()?)
        .map_err(port_error)
    }

    fn record_password_failure(&self, email: &str, ttl: u64) -> Result<u32, ApplicationError> {
        let ttl = failure_window(ttl)?;
        let key = self.digest_key("password-failures", email.as_bytes());
        redis::Script::new(
            "local count = redis.call('INCR', KEYS[1])
             if redis.call('TTL', KEYS[1]) < 0 then
                 redis.call('EXPIRE', KEYS[1], ARGV[1])
             end
             return count",
        )
        .key(key)
        .arg(ttl)
        .invoke(&mut self.connection()?)
        .map_err(port_error)
    }

    fn clear_password_failures(&self, email: &str) -> Result<(), ApplicationError> {
        let key = self.digest_key("password-failures", email.as_bytes());
        self.connection()?.del::<_, ()>(key).map_err(port_error)
    }

    fn claim_totp(&self, user_id: UserId, code: &str, ttl: u64) -> Result<bool, ApplicationError> {
        let mut claim = user_id.as_uuid().as_bytes().to_vec();
        claim.extend_from_slice(code.as_bytes());
        let key = self.digest_key("totp-used", &claim);
        let response: Option<String> = redis::cmd("SET")
            .arg(key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(ttl)
            .query(&mut self.connection()?)
            .map_err(port_error)?;
        Ok(response.is_some())
    }
}

fn failure_window(ttl: u64) -> Result<u32, ApplicationError> {
    u32::try_from(ttl)
        .ok()
        .filter(|ttl| *ttl > 0)
        .ok_or_else(|| ApplicationError::InvalidInput("invalid failure window".into()))
}

fn port_error(error: redis::RedisError) -> ApplicationError {
    ApplicationError::Port(format!("redis: {error}"))
}
