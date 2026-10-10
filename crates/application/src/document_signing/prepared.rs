use super::SigningAccountContext;
use crate::credential_trust::CredentialTrustSnapshot;
use domain::{
    clock::OffsetDateTime,
    crypto::{CredentialCertificate, CredentialCheck},
    document_signing::SigningBindingStatement,
};

/// Opaque public intention, with no successful possession or authority claim.
#[derive(Debug, Clone)]
pub struct PreparedSigningBinding {
    pub(super) context: SigningAccountContext,
    pub(super) statement: SigningBindingStatement,
    pub(super) certificate: CredentialCertificate,
    pub(super) trust: CredentialTrustSnapshot,
}
impl PreparedSigningBinding {
    pub fn context(&self) -> &SigningAccountContext {
        &self.context
    }
    pub fn statement(&self) -> &SigningBindingStatement {
        &self.statement
    }
    pub fn certificate(&self) -> &CredentialCertificate {
        &self.certificate
    }
    pub fn trust(&self) -> &CredentialTrustSnapshot {
        &self.trust
    }
}

/// A provisional cryptographic result whose mutable facts must be checked at commit.
pub struct VerifiedSigningBinding {
    pub(super) prepared: PreparedSigningBinding,
    pub(super) check: CredentialCheck,
    pub(super) not_before: OffsetDateTime,
}
impl VerifiedSigningBinding {
    pub fn prepared(&self) -> &PreparedSigningBinding {
        &self.prepared
    }
    pub fn check(&self) -> &CredentialCheck {
        &self.check
    }
    pub fn not_before(&self) -> OffsetDateTime {
        self.not_before
    }
}
