use domain::crypto::Sha256Digest;
use zeroize::Zeroizing;

use crate::{identity::validation::validate_password, ApplicationError};

use super::*;

mod owner;
mod pending;
mod recipient;
mod validation;

pub struct InvitationService {
    ports: InvitationPorts,
}

impl InvitationService {
    pub fn new(ports: InvitationPorts) -> Self {
        Self { ports }
    }

    fn token_digest(&self, token: &[u8]) -> Sha256Digest {
        let mut bytes = Zeroizing::new(b"qadra:invitation:v1\0".to_vec());
        bytes.extend_from_slice(token);
        self.ports.digests.hash_bytes(&bytes)
    }

    fn admit(
        &self,
        token: &[u8],
        operation: Uuid,
        password: &str,
    ) -> Result<Sha256Digest, ApplicationError> {
        if token.len() != 32 {
            return Err(ApplicationError::InvalidCredentials);
        }
        validation::operation(operation)?;
        validate_password(password)?;
        let digest = self.token_digest(token);
        if !self.ports.limiter.admit(digest)? {
            return Err(ApplicationError::InvalidCredentials);
        }
        Ok(digest)
    }

    /// The high-entropy capability protects these persisted intent digests from
    /// becoming public password or six-digit-code verifiers after database reads.
    fn intent_digest(
        &self,
        purpose: &[u8],
        token: &[u8],
        operation: Uuid,
        fields: &[&[u8]],
    ) -> Sha256Digest {
        let mut bytes = Zeroizing::new(purpose.to_vec());
        bytes.extend_from_slice(token);
        bytes.extend_from_slice(operation.as_bytes());
        for field in fields {
            bytes.extend_from_slice(&(field.len() as u64).to_be_bytes());
            bytes.extend_from_slice(field);
        }
        self.ports.digests.hash_bytes(&bytes)
    }
}

impl InvitationWorkflow for InvitationService {
    fn issue(
        &self,
        token: &str,
        request: InvitationIssueRequest,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.issue_invitation(token, request)
    }

    fn resend(
        &self,
        token: &str,
        id: InvitationId,
        request: InvitationResendRequest,
    ) -> Result<InvitationSummary, ApplicationError> {
        self.resend_invitation(token, id, request)
    }

    fn list(
        &self,
        token: &str,
        query: InvitationQuery,
    ) -> Result<InvitationPage, ApplicationError> {
        self.list_invitations(token, query)
    }

    fn get(&self, token: &str, id: InvitationId) -> Result<InvitationSummary, ApplicationError> {
        self.get_invitation(token, id)
    }

    fn prepare(
        &self,
        token: &[u8],
        operation_id: Uuid,
        password: &str,
    ) -> Result<InvitationEnrollment, ApplicationError> {
        self.prepare_enrollment(token, operation_id, password)
    }

    fn finish(
        &self,
        token: &[u8],
        operation_id: Uuid,
        password: &str,
        code: &str,
    ) -> Result<InvitationReceipt, ApplicationError> {
        self.finish_enrollment(token, operation_id, password, code)
    }

    fn receipt(
        &self,
        token: &[u8],
        operation_id: Uuid,
        password: &str,
    ) -> Result<Option<InvitationReceipt>, ApplicationError> {
        self.enrollment_receipt(token, operation_id, password)
    }
}
