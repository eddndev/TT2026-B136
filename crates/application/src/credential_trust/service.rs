use super::{
    CredentialTrustExpectation, CredentialTrustPublisher, CredentialTrustSnapshot,
    CredentialTrustStore,
};
use crate::ApplicationError;
use domain::{clock::Clock, crypto::InternalDeclarationVerifier};
use std::sync::Arc;

pub struct CredentialTrustService {
    store: Arc<dyn CredentialTrustStore>,
    verifier: Arc<dyn InternalDeclarationVerifier>,
    clock: Arc<dyn Clock + Send + Sync>,
}
impl CredentialTrustService {
    pub fn new(
        store: Arc<dyn CredentialTrustStore>,
        verifier: Arc<dyn InternalDeclarationVerifier>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Self {
        Self {
            store,
            verifier,
            clock,
        }
    }
}
impl CredentialTrustPublisher for CredentialTrustService {
    fn publish(
        &self,
        root: &[u8],
        crl: &[u8],
        expected: CredentialTrustExpectation,
    ) -> Result<CredentialTrustSnapshot, ApplicationError> {
        expected
            .next()
            .ok_or(ApplicationError::CredentialTrustRevisionExhausted)?;
        let inspection =
            self.verifier
                .inspect_trust(root, crl, self.clock.now().unix_timestamp())?;
        self.store.publish(expected, inspection)
    }
}
