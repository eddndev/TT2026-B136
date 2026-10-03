use super::{
    certificate_authority::mfa_error,
    mfa::MfaAdmission,
    session::{same_session, status},
    IdentityService,
};
use crate::{
    identity::{
        certificate_login::SessionAuthentication, Principal, SessionIdentity, SessionResult,
    },
    ApplicationError,
};

impl IdentityService {
    pub(super) fn issue_session(
        &self,
        admission: &MfaAdmission,
        action: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let user = &admission.user;
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
        self.check_mfa_authority(admission)?;
        let principal = Principal::from(&current);
        let identity = SessionIdentity {
            principal: principal.clone(),
            auth_generation: user.auth_generation,
        };
        let grant = match &admission.authentication {
            SessionAuthentication::Password => self
                .ports
                .sessions
                .create_session(&identity, self.session_policy)?,
            SessionAuthentication::Certificate(origin) => {
                let ceiling = origin
                    .valid_until_unix_seconds
                    .checked_mul(1000)
                    .ok_or(ApplicationError::MfaRejected)?;
                self.ports.sessions.create_certificate_session(
                    &identity,
                    origin,
                    self.session_policy,
                    ceiling,
                )?
            }
        };
        let admission_result = (|| {
            self.validate_session_state(&grant.state)
                .map_err(mfa_error)?;
            if grant.state.identity != identity
                || grant.state.authentication != admission.authentication
            {
                return Err(ApplicationError::MfaRejected);
            }
            let policy_ceiling = grant
                .state
                .server_now_unix_ms
                .checked_add(self.session_policy.absolute_ttl_seconds() as i64 * 1000)
                .ok_or(ApplicationError::MfaRejected)?;
            if grant.state.absolute_expires_at_unix_ms > policy_ceiling {
                return Err(ApplicationError::MfaRejected);
            }
            self.audit(&principal.email, action, &principal.id.to_string())?;
            if matches!(
                admission.authentication,
                SessionAuthentication::Certificate(_)
            ) {
                let confirmed = self.admit_session(&grant.access_token).map_err(mfa_error)?;
                if !same_session(&confirmed, &grant.state)
                    || admission
                        .expires_at
                        .is_none_or(|expires| self.now_seconds() >= expires)
                {
                    return Err(ApplicationError::MfaRejected);
                }
                Ok(confirmed)
            } else {
                Ok(grant.state.clone())
            }
        })();
        let state = match admission_result {
            Ok(state) => state,
            Err(error) => {
                self.ports
                    .sessions
                    .revoke_session(&grant.access_token)
                    .map_err(|_| {
                        ApplicationError::Port(
                            "session cleanup failed after rejected admission".into(),
                        )
                    })?;
                return Err(error);
            }
        };
        let session = status(state, self.session_policy);
        let deadline = session
            .idle_expires_at_unix_ms
            .unwrap_or(session.absolute_expires_at_unix_ms)
            .min(session.absolute_expires_at_unix_ms);
        let expires_in_seconds =
            deadline.saturating_sub(session.server_now_unix_ms).max(0) as u64 / 1000;
        Ok(SessionResult {
            access_token: grant.access_token,
            expires_in_seconds,
            session,
            principal,
        })
    }
}
