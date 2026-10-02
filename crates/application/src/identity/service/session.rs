//! Session admission, explicit activity, and MFA session issuance.

use super::IdentityService;
use crate::identity::{
    Principal, SessionIdentity, SessionPolicy, SessionResult, SessionState, SessionStatus,
    UserRecord,
};
use crate::ApplicationError;

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
        let updated = self
            .ports
            .sessions
            .record_activity(access_token, &admitted.identity, self.session_policy)?
            .filter(|state| state.identity == admitted.identity)
            .ok_or(ApplicationError::InvalidSession)?;
        Ok(status(updated, self.session_policy))
    }

    fn admit_session(&self, access_token: &str) -> Result<SessionState, ApplicationError> {
        let cached = self
            .ports
            .sessions
            .find_session(access_token, self.session_policy)?
            .ok_or(ApplicationError::InvalidSession)?;
        let Some(user) = self.ports.users.find_by_id(cached.identity.principal.id)? else {
            return Err(ApplicationError::InvalidSession);
        };
        if !user.active
            || user.auth_generation > i64::MAX as u64
            || cached.identity.auth_generation != user.auth_generation
            || cached.identity.principal != Principal::from(&user)
        {
            return Err(ApplicationError::InvalidSession);
        }
        self.ports
            .sessions
            .find_session(access_token, self.session_policy)?
            .filter(|confirmed| confirmed.identity == cached.identity)
            .ok_or(ApplicationError::InvalidSession)
    }

    pub(super) fn issue_session(
        &self,
        user: &UserRecord,
        action: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let current = self
            .ports
            .users
            .find_by_id(user.id)?
            .filter(|current| {
                current.active
                    && current.auth_generation <= i64::MAX as u64
                    && current.auth_generation == user.auth_generation
                    && Principal::from(current) == Principal::from(user)
            })
            .ok_or(ApplicationError::MfaRejected)?;
        let principal = Principal::from(&current);
        let grant = self.ports.sessions.create_session(
            &SessionIdentity {
                principal: principal.clone(),
                auth_generation: user.auth_generation,
            },
            self.session_policy,
        )?;
        let access_token = grant.access_token;
        if let Err(error) = self.audit(&principal.email, action, &principal.id.to_string()) {
            self.ports
                .sessions
                .revoke_session(&access_token)
                .map_err(|_| {
                    ApplicationError::Port("session cleanup failed after audit failure".into())
                })?;
            return Err(error);
        }
        let session = status(grant.state, self.session_policy);
        let deadline = session
            .idle_expires_at_unix_ms
            .unwrap_or(session.absolute_expires_at_unix_ms)
            .min(session.absolute_expires_at_unix_ms);
        let expires_in_seconds =
            deadline.saturating_sub(session.server_now_unix_ms).max(0) as u64 / 1000;
        Ok(SessionResult {
            access_token,
            expires_in_seconds,
            session,
            principal,
        })
    }
}

fn status(state: SessionState, policy: SessionPolicy) -> SessionStatus {
    SessionStatus {
        principal: state.identity.principal,
        policy,
        server_now_unix_ms: state.server_now_unix_ms,
        absolute_expires_at_unix_ms: state.absolute_expires_at_unix_ms,
        idle_expires_at_unix_ms: state.idle_expires_at_unix_ms,
    }
}
