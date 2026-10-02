use std::sync::Arc;

use super::{
    CompleteReset, IssueReset, PasswordResetDelivery, PasswordResetLimiter,
    PasswordResetRepository, ResetCompletion, ResetDeliveryOutcome, ResetEnvelope,
    ResetIssueOutcome, ResetPolicy, ResetRequestAccepted, ResetTokenSource,
};
use crate::identity::validation::{normalize_email, validate_password};
use crate::ApplicationError;
use domain::crypto::{DocumentHasher, PasswordHasher, Sha256Digest};
use zeroize::Zeroizing;

const TOKEN_BYTES: usize = 32;
const DIGEST_CONTEXT: &[u8] = b"qadra:password-reset:v1\0";

/// Internal dependencies. No user, session or MFA mutation ports are available.
pub struct PasswordResetPorts {
    pub repository: Arc<dyn PasswordResetRepository>,
    pub delivery: Arc<dyn PasswordResetDelivery>,
    pub limiter: Arc<dyn PasswordResetLimiter>,
    pub tokens: Arc<dyn ResetTokenSource>,
    pub digests: Arc<dyn DocumentHasher + Send + Sync>,
    pub passwords: Arc<dyn PasswordHasher + Send + Sync>,
}

/// Internal reset orchestration. Successful delivery is not an authenticated session.
pub struct PasswordResetService {
    ports: PasswordResetPorts,
    policy: ResetPolicy,
}

impl PasswordResetService {
    pub fn new(ports: PasswordResetPorts, policy: ResetPolicy) -> Self {
        Self { ports, policy }
    }

    pub fn request(&self, email: &str) -> Result<ResetRequestAccepted, ApplicationError> {
        let email = normalize_email(email)?;
        if !self.ports.limiter.admit_request(&email)? {
            return Ok(ResetRequestAccepted);
        }
        let token = self.ports.tokens.generate()?;
        let digest = self.digest(token.as_ref());
        let outcome = self.ports.repository.issue(IssueReset {
            email,
            digest,
            policy: self.policy,
        })?;
        if let ResetIssueOutcome::Issued(issue) = outcome {
            let issued_id = issue.id;
            let result = self.ports.delivery.deliver(ResetEnvelope {
                email: issue.email,
                expires_at: issue.expires_at,
                token,
            })?;
            if result == ResetDeliveryOutcome::DefinitelyRejected {
                self.ports
                    .repository
                    .cancel_undelivered(issued_id, digest)?;
            }
        }
        Ok(ResetRequestAccepted)
    }

    /// The caller retains ownership of the supplied token and password buffers.
    /// No raw token or password is copied into durable commands or errors.
    pub fn complete(
        &self,
        token: &[u8],
        password: &str,
    ) -> Result<ResetCompletion, ApplicationError> {
        if token.len() != TOKEN_BYTES {
            return Ok(ResetCompletion::Rejected);
        }
        validate_password(password)?;
        let digest = self.digest(token);
        if !self.ports.limiter.admit_completion(digest)? {
            return Ok(ResetCompletion::Rejected);
        }
        let Some(candidate) = self.ports.repository.inspect(digest)? else {
            return Ok(ResetCompletion::Rejected);
        };
        let password_hash = self.ports.passwords.hash(password)?;
        self.ports.repository.complete(CompleteReset {
            digest,
            candidate,
            password_hash,
        })
    }

    fn digest(&self, token: &[u8]) -> Sha256Digest {
        let mut input = Zeroizing::new(Vec::with_capacity(DIGEST_CONTEXT.len() + TOKEN_BYTES));
        input.extend_from_slice(DIGEST_CONTEXT);
        input.extend_from_slice(token);
        self.ports.digests.hash_bytes(&input)
    }
}
