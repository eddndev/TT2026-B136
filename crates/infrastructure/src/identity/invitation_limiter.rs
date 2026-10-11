//! Independent Redis attempt budgets for public invitation enrollment.
use application::{identity::invitations::InvitationLimiter, ApplicationError};
use domain::crypto::Sha256Digest;
use std::time::{Duration, Instant};

pub struct RedisInvitationLimiter {
    client: redis::Client,
    global: u32,
    per_token: u32,
    window_ms: u64,
    connect_timeout: Duration,
    io_timeout: Duration,
}

impl RedisInvitationLimiter {
    /// Handshake behavior follows the synchronous Redis driver. Commands are
    /// never retried after an uncertain response that may have spent the budget.
    pub fn connect(
        url: &str,
        global: u32,
        per_token: u32,
        window_seconds: u64,
        connect_timeout: Duration,
        io_timeout: Duration,
    ) -> Result<Self, ApplicationError> {
        if global == 0
            || per_token == 0
            || !(1..=86_400).contains(&window_seconds)
            || [connect_timeout, io_timeout]
                .iter()
                .any(|d| d.is_zero() || Instant::now().checked_add(*d).is_none())
        {
            return Err(ApplicationError::InvalidConfiguration(
                "invalid invitation attempt budgets or Redis timeouts".into(),
            ));
        }
        Ok(Self {
            client: redis::Client::open(url).map_err(|_| unavailable())?,
            global,
            per_token,
            window_ms: window_seconds * 1000,
            connect_timeout,
            io_timeout,
        })
    }
}

impl InvitationLimiter for RedisInvitationLimiter {
    fn admit(&self, digest: Sha256Digest) -> Result<bool, ApplicationError> {
        let mut connection = self
            .client
            .get_connection_with_timeout(self.connect_timeout)
            .map_err(|_| unavailable())?;
        connection
            .set_read_timeout(Some(self.io_timeout))
            .map_err(|_| unavailable())?;
        connection
            .set_write_timeout(Some(self.io_timeout))
            .map_err(|_| unavailable())?;
        let result: i64 = redis::Script::new(include_str!("password_reset_runtime/budget.lua"))
            .key("identity:invitation:v1:global")
            .key(format!("identity:invitation:v1:token:{}", digest.to_hex()))
            .arg(self.global)
            .arg(self.window_ms)
            .arg(self.per_token)
            .arg(self.window_ms)
            .invoke(&mut connection)
            .map_err(|_| unavailable())?;
        match result {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(unavailable()),
        }
    }
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("invitation attempt limiter unavailable".into())
}
