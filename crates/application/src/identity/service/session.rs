//! Session admission and activity preserve the exact authentication origin.

use super::{certificate_authority::session_error, IdentityService};
use crate::{
    identity::{
        certificate_login::SessionAuthentication, Principal, SessionPolicy, SessionState,
        SessionStatus,
    },
    ApplicationError,
};

impl IdentityService {
    pub fn authenticate(&self, access_token: &str) -> Result<Principal, ApplicationError> {
        Ok(self.admit_session(access_token)?.identity.principal)
    }

    pub fn session_status(&self, access_token: &str) -> Result<SessionStatus, ApplicationError> {
        Ok(status(
            self.admit_session(access_token)?,
            self.session_policy,
        ))
    }

    pub fn record_activity(&self, access_token: &str) -> Result<SessionStatus, ApplicationError> {
        let admitted = self.admit_session(access_token)?;
        let updated = match &admitted.authentication {
            SessionAuthentication::Password => self.ports.sessions.record_activity(
                access_token,
                &admitted.identity,
                self.session_policy,
            )?,
            SessionAuthentication::Certificate(origin) => {
                self.ports.sessions.record_certificate_activity(
                    access_token,
                    &admitted.identity,
                    origin,
                    self.session_policy,
                )?
            }
        }
        .filter(|value| same_session(value, &admitted))
        .ok_or(ApplicationError::InvalidSession)?;
        self.validate_session_state(&updated)?;
        let updated = if matches!(
            updated.authentication,
            SessionAuthentication::Certificate(_)
        ) {
            let confirmed = self.admit_session(access_token)?;
            if !same_session(&confirmed, &updated) {
                return Err(ApplicationError::InvalidSession);
            }
            confirmed
        } else {
            updated
        };
        Ok(status(updated, self.session_policy))
    }

    pub(super) fn admit_session(
        &self,
        access_token: &str,
    ) -> Result<SessionState, ApplicationError> {
        let cached = self
            .ports
            .sessions
            .find_session(access_token, self.session_policy)?
            .ok_or(ApplicationError::InvalidSession)?;
        self.validate_session_state(&cached)?;
        let user = self
            .ports
            .users
            .find_by_id(cached.identity.principal.id)?
            .ok_or(ApplicationError::InvalidSession)?;
        if !user.active
            || user.auth_generation > i64::MAX as u64
            || cached.identity.auth_generation != user.auth_generation
            || cached.identity.principal != Principal::from(&user)
        {
            return Err(ApplicationError::InvalidSession);
        }
        if let SessionAuthentication::Certificate(origin) = &cached.authentication {
            self.guard_certificate(origin).map_err(session_error)?;
        }
        let mut confirmed = self
            .ports
            .sessions
            .find_session(access_token, self.session_policy)?
            .filter(|value| same_session(value, &cached))
            .ok_or(ApplicationError::InvalidSession)?;
        if let SessionAuthentication::Certificate(origin) = &confirmed.authentication {
            self.guard_certificate(origin).map_err(session_error)?;
            let now = i64::try_from(
                self.ports
                    .clock
                    .now()
                    .unix_timestamp_nanos()
                    .div_euclid(1_000_000),
            )
            .map_err(|_| ApplicationError::InvalidSession)?;
            confirmed.server_now_unix_ms = confirmed.server_now_unix_ms.max(now);
        }
        self.validate_session_state(&confirmed)?;
        Ok(confirmed)
    }

    pub(super) fn validate_session_state(
        &self,
        value: &SessionState,
    ) -> Result<(), ApplicationError> {
        let now = value.server_now_unix_ms;
        let absolute = value.absolute_expires_at_unix_ms;
        if now < 0
            || absolute <= now
            || value.idle_expires_at_unix_ms.is_some()
                != self.session_policy.idle_ttl_seconds().is_some()
            || value
                .idle_expires_at_unix_ms
                .is_some_and(|idle| idle <= now || idle > absolute)
        {
            return Err(ApplicationError::InvalidSession);
        }
        if let SessionAuthentication::Certificate(origin) = &value.authentication {
            let ceiling = origin
                .valid_until_unix_seconds
                .checked_mul(1000)
                .ok_or(ApplicationError::InvalidSession)?;
            let begins = origin
                .valid_from_unix_seconds
                .checked_mul(1000)
                .ok_or(ApplicationError::InvalidSession)?;
            if value.identity.principal != origin.principal
                || value.identity.auth_generation != origin.auth_generation
                || now < begins
                || absolute > ceiling
            {
                return Err(ApplicationError::InvalidSession);
            }
        }
        Ok(())
    }
}

pub(super) fn same_session(left: &SessionState, right: &SessionState) -> bool {
    left.identity == right.identity
        && left.authentication == right.authentication
        && left.absolute_expires_at_unix_ms == right.absolute_expires_at_unix_ms
}

pub(super) fn status(state: SessionState, policy: SessionPolicy) -> SessionStatus {
    SessionStatus {
        principal: state.identity.principal,
        policy,
        server_now_unix_ms: state.server_now_unix_ms,
        absolute_expires_at_unix_ms: state.absolute_expires_at_unix_ms,
        idle_expires_at_unix_ms: state.idle_expires_at_unix_ms,
    }
}
