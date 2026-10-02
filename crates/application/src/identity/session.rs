//! Server-selected lifetime limits and authenticated session deadlines.

use super::{Principal, SessionIdentity};
use crate::ApplicationError;

/// Lifetime limits selected by the server, never by a bearer request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionPolicy {
    absolute_ttl_seconds: u64,
    idle_ttl_seconds: Option<u64>,
}

impl SessionPolicy {
    pub fn new(
        absolute_ttl_seconds: u64,
        idle_ttl_seconds: Option<u64>,
    ) -> Result<Self, ApplicationError> {
        if !(1..=86_400).contains(&absolute_ttl_seconds)
            || idle_ttl_seconds.is_some_and(|idle| idle == 0 || idle > absolute_ttl_seconds)
        {
            return Err(ApplicationError::InvalidConfiguration(
                "session lifetime must be positive, at most 86400 seconds, and idle cannot exceed absolute".into(),
            ));
        }
        Ok(Self {
            absolute_ttl_seconds,
            idle_ttl_seconds,
        })
    }

    pub fn absolute_ttl_seconds(self) -> u64 {
        self.absolute_ttl_seconds
    }

    pub fn idle_ttl_seconds(self) -> Option<u64> {
        self.idle_ttl_seconds
    }
}

/// Internal state returned by an atomic session-store operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub identity: SessionIdentity,
    pub server_now_unix_ms: i64,
    pub absolute_expires_at_unix_ms: i64,
    pub idle_expires_at_unix_ms: Option<i64>,
}

/// Newly issued bearer and its authoritative store deadlines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionGrant {
    pub access_token: String,
    pub state: SessionState,
}

/// Public session metadata; authentication generations remain internal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStatus {
    pub principal: Principal,
    pub policy: SessionPolicy,
    pub server_now_unix_ms: i64,
    pub absolute_expires_at_unix_ms: i64,
    pub idle_expires_at_unix_ms: Option<i64>,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            absolute_ttl_seconds: 86_400,
            idle_ttl_seconds: None,
        }
    }
}
