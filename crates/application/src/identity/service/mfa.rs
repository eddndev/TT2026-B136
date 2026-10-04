use domain::crypto::{RecoveryCodeOutcome, TotpVerification};

use super::{
    certificate_authority::mfa_error, IdentityService, CHALLENGE_TTL_SECONDS,
    TOTP_REPLAY_TTL_SECONDS,
};
use crate::{
    identity::{
        certificate_login::{MfaChallenge, SessionAuthentication},
        Principal, SessionResult, UserRecord,
    },
    ApplicationError,
};

pub(super) struct MfaAdmission {
    pub user: UserRecord,
    pub authentication: SessionAuthentication,
    pub expires_at: Option<i64>,
}

impl IdentityService {
    pub fn complete_totp(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let admission = self.take_challenge_user(challenge_token)?;
        let secret = self
            .ports
            .secrets
            .expose(admission.user.id, &admission.user.protected_totp_secret)?;
        let unix = self.now_seconds().max(0) as u64;
        let accepted = self.ports.totp.verify(&secret, code, unix)? == TotpVerification::Accepted
            && self
                .ports
                .sessions
                .claim_totp(admission.user.id, code, TOTP_REPLAY_TTL_SECONDS)?;
        if !accepted {
            return Err(ApplicationError::MfaRejected);
        }
        self.issue_session(&admission, "identity.totp_accepted")
    }

    pub fn complete_recovery(
        &self,
        challenge_token: &str,
        code: &str,
    ) -> Result<SessionResult, ApplicationError> {
        let mut admission = self.take_challenge_user(challenge_token)?;
        let expected_revision = admission.user.revision;
        if admission
            .user
            .recovery_codes
            .consume(code, self.ports.passwords.as_ref())?
            != RecoveryCodeOutcome::Accepted
        {
            return Err(ApplicationError::MfaRejected);
        }
        self.ports.users.replace_recovery_codes(
            admission.user.id,
            expected_revision,
            admission.user.recovery_codes.clone(),
            self.ports.clock.now(),
        )?;
        self.issue_session(&admission, "identity.recovery_accepted")
    }

    fn take_challenge_user(&self, token: &str) -> Result<MfaAdmission, ApplicationError> {
        let challenge = self
            .ports
            .sessions
            .take_mfa_challenge(token)?
            .ok_or(ApplicationError::MfaRejected)?;
        let (identity, authentication, expires_at) = match challenge {
            MfaChallenge::Password(value) => (value, SessionAuthentication::Password, None),
            MfaChallenge::Certificate(value) => {
                if value.identity.user_id != value.provenance.principal.id
                    || value.identity.auth_generation != value.provenance.auth_generation
                    || value.expires_at_unix_seconds > value.provenance.valid_until_unix_seconds
                {
                    return Err(ApplicationError::MfaRejected);
                }
                (
                    value.identity,
                    SessionAuthentication::Certificate(value.provenance.into()),
                    Some(value.expires_at_unix_seconds),
                )
            }
        };
        let user = self
            .ports
            .users
            .find_by_id(identity.user_id)?
            .filter(|user| {
                user.active
                    && user.auth_generation <= i64::MAX as u64
                    && user.auth_generation == identity.auth_generation
            })
            .ok_or(ApplicationError::MfaRejected)?;
        let admission = MfaAdmission {
            user,
            authentication,
            expires_at,
        };
        self.check_mfa_authority(&admission)?;
        Ok(admission)
    }

    pub(super) fn check_mfa_authority(
        &self,
        admission: &MfaAdmission,
    ) -> Result<(), ApplicationError> {
        if let SessionAuthentication::Certificate(origin) = &admission.authentication {
            if origin.principal != Principal::from(&admission.user)
                || origin.auth_generation != admission.user.auth_generation
            {
                return Err(ApplicationError::MfaRejected);
            }
            self.guard_certificate(origin).map_err(mfa_error)?;
            let now = self.now_seconds();
            if admission
                .expires_at
                .and_then(|expires| expires.checked_sub(now))
                .is_none_or(|remaining| !(1..=CHALLENGE_TTL_SECONDS as i64).contains(&remaining))
                || now < origin.valid_from_unix_seconds
                || now >= origin.valid_until_unix_seconds
            {
                return Err(ApplicationError::MfaRejected);
            }
        }
        Ok(())
    }
}
