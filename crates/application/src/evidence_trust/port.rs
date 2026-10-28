use super::EvidenceTrust;
use crate::credential_trust::CredentialTrustSnapshot;
use crate::ApplicationError;

/// Reads the newest published credential trust snapshot without changing it.
pub trait CredentialTrustReader: Send + Sync {
    fn current(&self) -> Result<Option<CredentialTrustSnapshot>, ApplicationError>;
}

/// Returns the current evidence trust to any authenticated principal.
pub trait EvidenceTrustWorkflow: Send + Sync {
    fn current(&self, access_token: &str) -> Result<EvidenceTrust, ApplicationError>;
}
