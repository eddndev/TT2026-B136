//! Admission for one separately enrolled CA operator, independent of office roles.
use std::sync::Arc;

use domain::identity::{Role, UserId};

use crate::identity::{
    normalize_email, IdentityService, LoginChallenge, MfaAttempt, MfaReason, Principal,
    SessionResult, SessionStatus,
};
use crate::ApplicationError;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct OperatorIdentity {
    pub id: UserId,
    pub email: String,
}

impl OperatorIdentity {
    pub fn new(id: UserId, email: &str) -> Result<Self, ApplicationError> {
        if id.as_uuid().is_nil() {
            return Err(ApplicationError::InvalidConfiguration(
                "CA operator is missing".into(),
            ));
        }
        Ok(Self {
            id,
            email: normalize_email(email)?,
        })
    }
}

pub struct OperatorSession {
    pub access_token: String,
    pub expires_in_seconds: u64,
    pub session: SessionStatus,
    pub operator: OperatorIdentity,
}

/// Only the identity and reason are loggable; the result contains credentials.
pub struct CaMfaAttempt {
    pub user_id: Option<UserId>,
    pub reason: MfaReason,
    pub result: Result<OperatorSession, ApplicationError>,
}

pub struct CaOperatorAccess {
    identity: Arc<IdentityService>,
    operator: OperatorIdentity,
}

impl CaOperatorAccess {
    pub fn new(identity: Arc<IdentityService>, operator: OperatorIdentity) -> Self {
        Self { identity, operator }
    }

    pub fn start_login(
        &self,
        email: &str,
        password: &str,
    ) -> Result<LoginChallenge, ApplicationError> {
        if normalize_email(email)? != self.operator.email {
            return Err(ApplicationError::InvalidCredentials);
        }
        self.identity.start_login(email, password)
    }

    pub fn complete_totp(
        &self,
        challenge: &str,
        code: &str,
    ) -> Result<OperatorSession, ApplicationError> {
        self.complete_totp_observed(challenge, code).result
    }

    pub fn complete_recovery(
        &self,
        challenge: &str,
        code: &str,
    ) -> Result<OperatorSession, ApplicationError> {
        self.complete_recovery_observed(challenge, code).result
    }

    pub fn complete_totp_observed(&self, challenge: &str, code: &str) -> CaMfaAttempt {
        self.finish_observed(self.identity.complete_totp_observed(challenge, code))
    }

    pub fn complete_recovery_observed(&self, challenge: &str, code: &str) -> CaMfaAttempt {
        self.finish_observed(self.identity.complete_recovery_observed(challenge, code))
    }

    pub fn authenticate(&self, token: &str) -> Result<OperatorIdentity, ApplicationError> {
        self.admit(&self.identity.authenticate(token)?)?;
        Ok(self.operator.clone())
    }

    pub fn session_status(&self, token: &str) -> Result<SessionStatus, ApplicationError> {
        self.authenticate(token)?;
        self.identity.session_status(token)
    }

    pub fn record_activity(&self, token: &str) -> Result<SessionStatus, ApplicationError> {
        self.authenticate(token)?;
        let status = self.identity.record_activity(token)?;
        self.authenticate(token)?;
        Ok(status)
    }

    pub fn logout(&self, token: &str) -> Result<(), ApplicationError> {
        self.authenticate(token)?;
        self.identity.logout(token)
    }

    fn admit(&self, principal: &Principal) -> Result<(), ApplicationError> {
        // Client is only a compatibility codec for separate CA credentials.
        // Authority comes from the exact configured identity, never this role.
        if principal.id != self.operator.id
            || principal.email != self.operator.email
            || principal.role != Role::Client
        {
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(())
    }

    fn finish(&self, session: SessionResult) -> Result<OperatorSession, ApplicationError> {
        if self.admit(&session.principal).is_err() {
            self.identity.logout(&session.access_token)?;
            return Err(ApplicationError::PermissionDenied);
        }
        Ok(OperatorSession {
            access_token: session.access_token,
            expires_in_seconds: session.expires_in_seconds,
            session: session.session,
            operator: self.operator.clone(),
        })
    }

    fn finish_observed(&self, attempt: MfaAttempt) -> CaMfaAttempt {
        let mut reason = attempt.reason;
        let result = attempt.result.and_then(|session| self.finish(session));
        if reason == MfaReason::Accepted {
            reason = match &result {
                Ok(_) => MfaReason::Accepted,
                Err(ApplicationError::PermissionDenied) => MfaReason::SessionAdmissionRejected,
                Err(_) => MfaReason::OperationalError,
            };
        }
        CaMfaAttempt {
            user_id: attempt.user_id,
            reason,
            result,
        }
    }
}
