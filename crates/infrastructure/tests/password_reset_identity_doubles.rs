//! Local delivery capture; reset entropy and transport are outside this test.

use std::sync::Mutex;

use application::identity::password_reset::{
    PasswordResetDelivery, PasswordResetLimiter, ResetDeliveryOutcome, ResetEnvelope,
    ResetTokenSource,
};
use application::ApplicationError;
use domain::crypto::Sha256Digest;
use zeroize::Zeroizing;

#[derive(Default)]
pub struct CapturedDelivery(Mutex<Option<ResetEnvelope>>);

impl CapturedDelivery {
    pub fn take(&self) -> ResetEnvelope {
        self.0
            .lock()
            .unwrap()
            .take()
            .expect("reset must reach local delivery")
    }
}

impl PasswordResetDelivery for CapturedDelivery {
    fn deliver(&self, envelope: ResetEnvelope) -> Result<ResetDeliveryOutcome, ApplicationError> {
        let mut captured = self.0.lock().unwrap();
        assert!(captured.is_none(), "unexpected duplicate reset delivery");
        *captured = Some(envelope);
        Ok(ResetDeliveryOutcome::Accepted)
    }
}

/// Deterministic fixture material does not verify production reset entropy.
pub struct FixedResetToken;

impl ResetTokenSource for FixedResetToken {
    fn generate(&self) -> Result<Zeroizing<[u8; 32]>, ApplicationError> {
        Ok(Zeroizing::new([37; 32]))
    }
}

/// Distributed rate limits have their own contract; this flow stays admitted.
pub struct AdmitReset;

impl PasswordResetLimiter for AdmitReset {
    fn admit_request(&self, _email: &str) -> Result<bool, ApplicationError> {
        Ok(true)
    }

    fn admit_completion(&self, _digest: Sha256Digest) -> Result<bool, ApplicationError> {
        Ok(true)
    }
}
