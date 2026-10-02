use std::time::{Duration, Instant};

use application::identity::password_reset::PasswordResetLimiter;
use application::ApplicationError;
use domain::crypto::{DocumentHasher, Sha256Digest};
use zeroize::Zeroizing;

use super::{PasswordResetRateLimit, PasswordResetRatePolicy};
use crate::RingSha256Hasher;

const REQUEST_GLOBAL: &str = "identity:password-reset:v1:request:global";
const COMPLETION_GLOBAL: &str = "identity:password-reset:v1:completion:global";

/// Shared fixed-window reset budgets, separate from login and session state.
pub struct RedisPasswordResetLimiter {
    client: redis::Client,
    policy: PasswordResetRatePolicy,
    connect_timeout: Duration,
    io_timeout: Duration,
}

impl RedisPasswordResetLimiter {
    /// Configures TCP connection and established-socket I/O deadlines.
    ///
    /// The synchronous driver performs authentication, database selection and
    /// client identification before socket I/O timeouts can be set. These
    /// settings do not bound those handshake reads. No command is retried after
    /// an uncertain reply; an error may follow an already consumed budget.
    pub fn connect(
        url: &str,
        policy: PasswordResetRatePolicy,
        connect_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Self, ApplicationError> {
        let now = Instant::now();
        if [connect_timeout, io_timeout]
            .iter()
            .any(|timeout| timeout.is_zero() || now.checked_add(*timeout).is_none())
        {
            return Err(ApplicationError::InvalidConfiguration(
                "password reset Redis timeouts must be positive and representable".into(),
            ));
        }
        Ok(Self {
            client: redis::Client::open(url).map_err(|_| unavailable())?,
            policy,
            connect_timeout,
            io_timeout,
        })
    }

    fn connection(&self) -> Result<redis::Connection, ApplicationError> {
        let connection = self
            .client
            .get_connection_with_timeout(self.connect_timeout)
            .map_err(|_| unavailable())?;
        connection
            .set_read_timeout(Some(self.io_timeout))
            .map_err(|_| unavailable())?;
        connection
            .set_write_timeout(Some(self.io_timeout))
            .map_err(|_| unavailable())?;
        Ok(connection)
    }

    fn admit(
        &self,
        global_key: &str,
        subject_key: String,
        global: PasswordResetRateLimit,
        subject: PasswordResetRateLimit,
    ) -> Result<bool, ApplicationError> {
        let admitted: i64 = redis::Script::new(include_str!("budget.lua"))
            .key(global_key)
            .key(subject_key)
            .arg(global.maximum)
            .arg(global.window_ms)
            .arg(subject.maximum)
            .arg(subject.window_ms)
            .invoke(&mut self.connection()?)
            .map_err(|_| unavailable())?;
        match admitted {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(unavailable()),
        }
    }
}

impl PasswordResetLimiter for RedisPasswordResetLimiter {
    fn admit_request(&self, normalized_email: &str) -> Result<bool, ApplicationError> {
        self.admit(
            REQUEST_GLOBAL,
            subject_key(
                "request:email",
                b"qadra:password-reset-request-limit:v1\0",
                normalized_email.as_bytes(),
            ),
            self.policy.request_global,
            self.policy.request_email,
        )
    }

    fn admit_completion(&self, digest: Sha256Digest) -> Result<bool, ApplicationError> {
        self.admit(
            COMPLETION_GLOBAL,
            subject_key(
                "completion:digest",
                b"qadra:password-reset-completion-limit:v1\0",
                digest.as_bytes(),
            ),
            self.policy.completion_global,
            self.policy.completion_digest,
        )
    }
}

fn subject_key(kind: &str, context: &[u8], subject: &[u8]) -> String {
    let mut bytes = Zeroizing::new(context.to_vec());
    bytes.extend_from_slice(subject);
    format!(
        "identity:password-reset:v1:{kind}:{}",
        RingSha256Hasher.hash_bytes(&bytes).to_hex()
    )
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("password reset rate limiter unavailable".into())
}
