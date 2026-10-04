use std::sync::Arc;

use domain::{
    crypto::{CredentialCheck, DocumentHasher, Signature},
    identity::UserId,
    owner_certificate_login::{LoginNonce, LoginStatement},
    owner_certificates::Uuid,
};

use super::{CertificateLoginChallenge, CertificateLoginContext, StoredCertificateLogin};
use crate::{
    credential_trust::CredentialTrustSnapshot, identity::LoginChallenge, ApplicationError,
};

/// Public certificate first factor, separate from password and mandatory MFA.
pub trait OwnerLoginWorkflow: Send + Sync {
    /// Captures the explicitly selected Owner binding without issuing a session.
    fn start_certificate_login(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<CertificateLoginChallenge, ApplicationError>;

    /// Consumes one proof and returns only a challenge for the existing MFA flow.
    fn prove_certificate_login(
        &self,
        token: &str,
        signature: &[u8],
    ) -> Result<LoginChallenge, ApplicationError>;
}

/// Loads a consistent active Owner, live binding and current published trust.
pub trait OwnerLoginAuthority: Send + Sync {
    /// None is known absence, never a substitute for a failed authority query.
    fn load(
        &self,
        owner: UserId,
        binding: Uuid,
    ) -> Result<Option<CertificateLoginContext>, ApplicationError>;
}

/// Bounded first-factor admission, unpredictable nonces and one-use captures.
pub trait OwnerLoginRuntime: Send + Sync {
    fn admit_start(&self, owner: UserId, binding: Uuid) -> Result<(), ApplicationError>;
    fn admit_proof(&self, token: &str) -> Result<(), ApplicationError>;
    fn nonce(&self) -> Result<LoginNonce, ApplicationError>;
    fn create(
        &self,
        value: &StoredCertificateLogin,
        ttl_seconds: u64,
    ) -> Result<String, ApplicationError>;
    /// Removes the capture atomically; absent, expired and consumed return None.
    fn take(&self, token: &str) -> Result<Option<StoredCertificateLogin>, ApplicationError>;
}

/// Verifies the login purpose, exact signature, Partner chain, CRL and time.
pub trait OwnerLoginVerifier: Send + Sync {
    fn verify_login(
        &self,
        statement: &LoginStatement,
        certificate: &[u8],
        signature: &Signature,
        trust: &CredentialTrustSnapshot,
        at: i64,
    ) -> Result<CredentialCheck, ApplicationError>;
}

/// Explicit opt-in capabilities; existing password construction stays unchanged.
pub struct CertificateLoginPorts {
    pub authority: Arc<dyn OwnerLoginAuthority>,
    pub runtime: Arc<dyn OwnerLoginRuntime>,
    pub verifier: Arc<dyn OwnerLoginVerifier>,
    pub hasher: Arc<dyn DocumentHasher + Send + Sync>,
}
