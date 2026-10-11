use domain::{
    crypto::{PasswordVerification, RecoveryCodeSet, RECOVERY_CODE_COUNT},
    identity::{Role, UserId},
};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use super::super::*;
use super::{validation, InvitationService};
use crate::{identity::UserRecord, ApplicationError};

const PREPARE_CONTEXT: &[u8] = b"qadra:invitation:prepare:v1\0";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PendingMaterial {
    format_version: u8,
    id: InvitationId,
    revision: u64,
    operation_id: Uuid,
    email: String,
    role: Role,
    expires_at: i64,
    user_id: UserId,
    password_hash: String,
    pub(super) totp_secret: Vec<u8>,
    totp_secret_base32: String,
    otpauth_uri: String,
    recovery_codes: Vec<String>,
    recovery_hashes: Vec<String>,
}

impl Drop for PendingMaterial {
    fn drop(&mut self) {
        self.password_hash.zeroize();
        self.totp_secret.zeroize();
        self.totp_secret_base32.zeroize();
        self.otpauth_uri.zeroize();
        self.recovery_codes.zeroize();
        self.recovery_hashes.zeroize();
    }
}

impl PendingMaterial {
    fn validate(
        &self,
        candidate: &InvitationCandidate,
        operation: Uuid,
    ) -> Result<(), ApplicationError> {
        if self.format_version != 1
            || self.id != candidate.id
            || self.revision != candidate.revision
            || self.operation_id != operation
            || self.email != candidate.email
            || self.role != candidate.role
            || self.expires_at != candidate.expires_at.unix_timestamp()
            || self.user_id.as_uuid().is_nil()
            || !(1..=4096).contains(&self.password_hash.len())
            || !(16..=128).contains(&self.totp_secret.len())
            || !(1..=256).contains(&self.totp_secret_base32.len())
            || !(1..=2048).contains(&self.otpauth_uri.len())
            || self.recovery_codes.len() != RECOVERY_CODE_COUNT
            || self.recovery_hashes.len() != RECOVERY_CODE_COUNT
            || self
                .recovery_codes
                .iter()
                .any(|value| !(1..=256).contains(&value.len()))
            || self
                .recovery_hashes
                .iter()
                .any(|value| !(1..=4096).contains(&value.len()))
        {
            return Err(validation::stored());
        }
        Ok(())
    }

    pub(super) fn enrollment(&self, candidate: &InvitationCandidate) -> InvitationEnrollment {
        InvitationEnrollment {
            id: self.id,
            revision: self.revision,
            operation_id: self.operation_id,
            email: self.email.clone(),
            role: self.role,
            expires_at: candidate.expires_at,
            totp_secret_base32: Zeroizing::new(self.totp_secret_base32.clone()),
            otpauth_uri: Zeroizing::new(self.otpauth_uri.clone()),
            recovery_codes: self
                .recovery_codes
                .iter()
                .cloned()
                .map(Zeroizing::new)
                .collect(),
        }
    }

    pub(super) fn user(
        &self,
        protected_totp_secret: Vec<u8>,
    ) -> Result<UserRecord, ApplicationError> {
        Ok(UserRecord {
            id: self.user_id,
            email: self.email.clone(),
            role: self.role,
            active: true,
            password_hash: self.password_hash.clone(),
            protected_totp_secret,
            recovery_codes: RecoveryCodeSet::from_hashes(self.recovery_hashes.clone())?,
            revision: 0,
            auth_generation: 0,
        })
    }

    pub(super) fn user_id(&self) -> UserId {
        self.user_id
    }
}

impl InvitationService {
    fn prepare_digest(
        &self,
        token: &[u8],
        operation: Uuid,
        password: &str,
    ) -> domain::crypto::Sha256Digest {
        self.intent_digest(PREPARE_CONTEXT, token, operation, &[password.as_bytes()])
    }

    pub(super) fn new_pending(
        &self,
        candidate: &InvitationCandidate,
        token: &[u8],
        operation: Uuid,
        password: &str,
    ) -> Result<PendingInvitation, ApplicationError> {
        let password_hash = Zeroizing::new(self.ports.passwords.hash(password)?);
        let totp = self.ports.totp.enroll(&candidate.email)?;
        let codes = self.ports.recovery.generate(RECOVERY_CODE_COUNT)?;
        if codes.len() != RECOVERY_CODE_COUNT {
            return Err(validation::stored());
        }
        let mut hashes = Zeroizing::new(Vec::with_capacity(RECOVERY_CODE_COUNT));
        for code in &codes {
            hashes.push(self.ports.passwords.hash(code)?);
        }
        let material = PendingMaterial {
            format_version: 1,
            id: candidate.id,
            revision: candidate.revision,
            operation_id: operation,
            email: candidate.email.clone(),
            role: candidate.role,
            expires_at: candidate.expires_at.unix_timestamp(),
            user_id: UserId::new(),
            password_hash: password_hash.to_string(),
            totp_secret: totp.secret.to_vec(),
            totp_secret_base32: totp.secret_base32.to_string(),
            otpauth_uri: totp.otpauth_uri.to_string(),
            recovery_codes: codes.iter().map(|value| value.to_string()).collect(),
            recovery_hashes: hashes.to_vec(),
        };
        material.validate(candidate, operation)?;
        let bytes =
            Zeroizing::new(serde_json::to_vec(&material).map_err(|_| validation::stored())?);
        if bytes.len() > MAX_INVITATION_PAYLOAD_BYTES {
            return Err(validation::stored());
        }
        let protected_material = self.ports.protector.protect(context(candidate), &bytes)?;
        validation::ciphertext(&protected_material)?;
        Ok(PendingInvitation {
            operation_id: operation,
            request_digest: self.prepare_digest(token, operation, password),
            protected_material,
        })
    }

    pub(super) fn open_pending(
        &self,
        candidate: &InvitationCandidate,
        pending: &PendingInvitation,
        token: &[u8],
        password: &str,
    ) -> Result<PendingMaterial, ApplicationError> {
        validation::operation(pending.operation_id).map_err(|_| validation::stored())?;
        validation::ciphertext(&pending.protected_material)?;
        let bytes = self
            .ports
            .protector
            .expose(context(candidate), &pending.protected_material)?;
        if bytes.len() > MAX_INVITATION_PAYLOAD_BYTES {
            return Err(validation::stored());
        }
        let material: PendingMaterial =
            serde_json::from_slice(&bytes).map_err(|_| validation::stored())?;
        material.validate(candidate, pending.operation_id)?;
        if self
            .ports
            .passwords
            .verify(password, &material.password_hash)?
            != PasswordVerification::Match
        {
            return Err(ApplicationError::InvalidCredentials);
        }
        if self.prepare_digest(token, pending.operation_id, password) != pending.request_digest {
            return Err(validation::stored());
        }
        Ok(material)
    }

    pub(super) fn confirm_pending(
        &self,
        candidate: &InvitationCandidate,
        pending: &PendingInvitation,
        digest: domain::crypto::Sha256Digest,
    ) -> Result<(), ApplicationError> {
        let current = self
            .ports
            .store
            .inspect(digest)?
            .ok_or(ApplicationError::InvalidCredentials)?;
        validation::candidate(&current, self.ports.clock.now())?;
        let present = current
            .pending
            .as_ref()
            .ok_or(ApplicationError::InvalidCredentials)?;
        if current.id != candidate.id
            || current.revision != candidate.revision
            || current.email != candidate.email
            || current.role != candidate.role
            || current.expires_at != candidate.expires_at
            || present.operation_id != pending.operation_id
            || present.request_digest != pending.request_digest
            || present.protected_material != pending.protected_material
        {
            return Err(ApplicationError::InvalidCredentials);
        }
        Ok(())
    }
}

fn context(candidate: &InvitationCandidate) -> InvitationProtectionContext {
    InvitationProtectionContext {
        purpose: InvitationPurpose::PendingMfa,
        id: candidate.id,
        revision: candidate.revision,
    }
}
