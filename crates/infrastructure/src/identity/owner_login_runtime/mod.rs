//! One-use public Owner login captures and independent first-factor budgets.

mod capture;
mod codec;
mod context;
mod limiter;
mod policy;
mod validation;
mod wire;

use std::time::{Duration, Instant};

use application::identity::certificate_login::{OwnerLoginRuntime, StoredCertificateLogin};
use application::ApplicationError;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use domain::{identity::UserId, owner_certificate_login::LoginNonce};
use uuid::Uuid;
use zeroize::Zeroizing;

pub use policy::{OwnerLoginRateLimit, OwnerLoginRatePolicy};

/// Redis-backed proof admission; password, MFA and session state stay separate.
pub struct RedisOwnerLoginRuntime {
    client: redis::Client,
    policy: OwnerLoginRatePolicy,
    connect_timeout: Duration,
    io_timeout: Duration,
}

impl RedisOwnerLoginRuntime {
    /// Configures connection and established-socket timeouts, with no defaults.
    /// The synchronous driver's authentication and database-selection handshake
    /// precedes socket timeout assignment. These are not total request deadlines.
    /// Uncertain writes are never retried by this adapter.
    pub fn connect(
        url: &str,
        policy: OwnerLoginRatePolicy,
        connect_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Self, ApplicationError> {
        let now = Instant::now();
        if [connect_timeout, io_timeout]
            .iter()
            .any(|timeout| timeout.is_zero() || now.checked_add(*timeout).is_none())
        {
            return Err(ApplicationError::InvalidConfiguration(
                "certificate login Redis timeouts must be positive and representable".into(),
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
}

impl OwnerLoginRuntime for RedisOwnerLoginRuntime {
    fn admit_start(&self, owner: UserId, binding: Uuid) -> Result<(), ApplicationError> {
        self.start_budget(owner, binding)
    }

    fn admit_proof(&self, token: &str) -> Result<(), ApplicationError> {
        token_shape(token)?;
        self.proof_budget(token)
    }

    fn nonce(&self) -> Result<LoginNonce, ApplicationError> {
        LoginNonce::from_bytes(random_bytes()?.as_ref()).map_err(|_| unavailable())
    }

    fn create(
        &self,
        value: &StoredCertificateLogin,
        ttl_seconds: u64,
    ) -> Result<String, ApplicationError> {
        self.create_capture(value, ttl_seconds)
    }

    fn take(&self, token: &str) -> Result<Option<StoredCertificateLogin>, ApplicationError> {
        token_shape(token)?;
        self.take_capture(token)
    }
}

fn random_bytes() -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
    let mut bytes = Zeroizing::new([0; 32]);
    getrandom::getrandom(bytes.as_mut()).map_err(|_| unavailable())?;
    Ok(bytes)
}

fn token_shape(token: &str) -> Result<(), ApplicationError> {
    if token.len() != 43 {
        return Err(invalid());
    }
    let bytes = URL_SAFE_NO_PAD.decode(token).map_err(|_| invalid())?;
    if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != token {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ApplicationError {
    ApplicationError::InvalidCredentials
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("certificate login runtime unavailable".into())
}
