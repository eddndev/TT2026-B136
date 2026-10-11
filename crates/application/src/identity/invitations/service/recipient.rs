use domain::crypto::{PasswordVerification, Sha256Digest, TotpVerification};

use super::super::*;
use super::{validation, InvitationService};
use crate::ApplicationError;

const FINISH_CONTEXT: &[u8] = b"qadra:invitation:finish:v1\0";

impl InvitationService {
    fn candidate(&self, digest: Sha256Digest) -> Result<InvitationCandidate, ApplicationError> {
        let candidate = self
            .ports
            .store
            .inspect(digest)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        validation::candidate(&candidate, self.ports.clock.now())?;
        Ok(candidate)
    }

    pub(super) fn prepare_enrollment(
        &self,
        token: &[u8],
        operation: Uuid,
        password: &str,
    ) -> Result<InvitationEnrollment, ApplicationError> {
        let digest = self.admit(token, operation, password)?;
        let candidate = self.candidate(digest)?;
        let pending = match &candidate.pending {
            Some(value) => value.clone(),
            None => {
                let proposed = self.new_pending(&candidate, token, operation, password)?;
                validation::candidate(&candidate, self.ports.clock.now())?;
                self.ports.store.save_pending(PrepareInvitation {
                    id: candidate.id,
                    revision: candidate.revision,
                    token_digest: digest,
                    pending: proposed,
                    at: self.ports.clock.now(),
                })?
            }
        };
        let material = self.open_pending(&candidate, &pending, token, password)?;
        self.confirm_pending(&candidate, &pending, digest)?;
        Ok(material.enrollment(&candidate))
    }

    pub(super) fn finish_enrollment(
        &self,
        token: &[u8],
        operation: Uuid,
        password: &str,
        code: &str,
    ) -> Result<InvitationReceipt, ApplicationError> {
        let digest = self.admit(token, operation, password)?;
        if code.len() != 6 || !code.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ApplicationError::MfaRejected);
        }
        let request_digest = self.intent_digest(
            FINISH_CONTEXT,
            token,
            operation,
            &[password.as_bytes(), code.as_bytes()],
        );
        if let Some(completed) = self.ports.store.completed(digest, operation)? {
            self.check_receipt(&completed.receipt, operation, password)?;
            if completed.request_digest != request_digest {
                return Err(ApplicationError::ConcurrentModification);
            }
            return Ok(completed.receipt);
        }
        let candidate = self.candidate(digest)?;
        let pending = candidate
            .pending
            .as_ref()
            .ok_or(ApplicationError::InvalidCredentials)?;
        let material = self.open_pending(&candidate, pending, token, password)?;
        let at = self.ports.clock.now();
        validation::candidate(&candidate, at)?;
        let seconds =
            u64::try_from(at.unix_timestamp()).map_err(|_| ApplicationError::InvalidCredentials)?;
        if self
            .ports
            .totp
            .verify(&material.totp_secret, code, seconds)?
            != TotpVerification::Accepted
        {
            return Err(ApplicationError::MfaRejected);
        }
        let protected = self
            .ports
            .secrets
            .protect(material.user_id(), &material.totp_secret)?;
        validation::ciphertext(&protected)?;
        validation::candidate(&candidate, self.ports.clock.now())?;
        let receipt = self.ports.store.finish(FinishInvitation {
            id: candidate.id,
            revision: candidate.revision,
            pending_operation_id: pending.operation_id,
            operation_id: operation,
            request_digest,
            token_digest: digest,
            user: material.user(protected)?,
            at: self.ports.clock.now(),
        })?;
        validation::receipt(&receipt, operation, self.ports.clock.now())?;
        if receipt.id != candidate.id
            || receipt.revision != candidate.revision
            || receipt.user_id != material.user_id()
            || receipt.email != candidate.email
            || receipt.role != candidate.role
            || receipt.accepted_at >= candidate.expires_at
        {
            return Err(validation::stored());
        }
        Ok(receipt)
    }

    pub(super) fn enrollment_receipt(
        &self,
        token: &[u8],
        operation: Uuid,
        password: &str,
    ) -> Result<Option<InvitationReceipt>, ApplicationError> {
        let digest = self.admit(token, operation, password)?;
        let Some(completed) = self.ports.store.completed(digest, operation)? else {
            return Ok(None);
        };
        self.check_receipt(&completed.receipt, operation, password)?;
        Ok(Some(completed.receipt))
    }

    fn check_receipt(
        &self,
        receipt: &InvitationReceipt,
        operation: Uuid,
        password: &str,
    ) -> Result<(), ApplicationError> {
        validation::receipt(receipt, operation, self.ports.clock.now())?;
        let user = self
            .ports
            .users
            .find_by_id(receipt.user_id)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        if user.id != receipt.user_id
            || user.email != receipt.email
            || self.ports.passwords.verify(password, &user.password_hash)?
                != PasswordVerification::Match
        {
            return Err(ApplicationError::InvalidCredentials);
        }
        let current = self
            .ports
            .users
            .find_by_id(user.id)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        if current.id != user.id
            || current.email != user.email
            || current.password_hash != user.password_hash
            || current.auth_generation != user.auth_generation
        {
            return Err(ApplicationError::InvalidCredentials);
        }
        Ok(())
    }
}
