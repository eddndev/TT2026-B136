//! Sanitized MFA outcomes for delivery-adapter access logs.

use super::SessionResult;
use crate::ApplicationError;
use domain::identity::UserId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MfaReason {
    Accepted,
    InvalidCodeOrOutsideWindow,
    CodeAlreadyUsed,
    ChallengeExpiredConsumedOrUnknown,
    AccountUnavailable,
    AccountInactive,
    CredentialsChanged,
    CertificateAuthorityInvalidOrExpired,
    SessionAdmissionRejected,
    RecoveryInvalidOrUsed,
    RejectionUnspecified,
    OperationalError,
}

impl MfaReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::InvalidCodeOrOutsideWindow => "invalid_code_or_outside_window",
            Self::CodeAlreadyUsed => "code_already_used",
            Self::ChallengeExpiredConsumedOrUnknown => "challenge_expired_consumed_or_unknown",
            Self::AccountUnavailable => "account_unavailable",
            Self::AccountInactive => "account_inactive",
            Self::CredentialsChanged => "credentials_changed",
            Self::CertificateAuthorityInvalidOrExpired => {
                "certificate_authority_invalid_or_expired"
            }
            Self::SessionAdmissionRejected => "session_admission_rejected",
            Self::RecoveryInvalidOrUsed => "recovery_invalid_or_used",
            Self::RejectionUnspecified => "rejection_unspecified",
            Self::OperationalError => "operational_error",
        }
    }
}

/// Only user_id and reason are safe to log; result contains session credentials.
pub struct MfaAttempt {
    pub user_id: Option<UserId>,
    pub reason: MfaReason,
    pub result: Result<SessionResult, ApplicationError>,
}

impl MfaAttempt {
    pub fn unspecified(result: Result<SessionResult, ApplicationError>) -> Self {
        Self::finish(None, MfaReason::RejectionUnspecified, result)
    }

    pub(crate) fn finish(
        user_id: Option<UserId>,
        reason: MfaReason,
        result: Result<SessionResult, ApplicationError>,
    ) -> Self {
        let (user_id, reason) = match &result {
            Ok(session) => (Some(session.principal.id), MfaReason::Accepted),
            Err(ApplicationError::MfaRejected) => (user_id, reason),
            Err(_) => (user_id, MfaReason::OperationalError),
        };
        Self {
            user_id,
            reason,
            result,
        }
    }
}
