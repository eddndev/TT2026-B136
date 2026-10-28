use std::sync::Arc;

use super::{CredentialTrustReader, EvidenceTrust, EvidenceTrustWorkflow, ServedEvidenceMaterial};
use crate::identity::IdentityWorkflow;
use crate::ApplicationError;

/// Authenticates before reading; without a reader individual signing trust is absent.
pub struct EvidenceTrustService {
    identity: Arc<dyn IdentityWorkflow>,
    reader: Option<Arc<dyn CredentialTrustReader>>,
    material: Arc<ServedEvidenceMaterial>,
}

impl EvidenceTrustService {
    pub fn new(
        identity: Arc<dyn IdentityWorkflow>,
        reader: Option<Arc<dyn CredentialTrustReader>>,
        material: Arc<ServedEvidenceMaterial>,
    ) -> Self {
        Self {
            identity,
            reader,
            material,
        }
    }
}

impl EvidenceTrustWorkflow for EvidenceTrustService {
    fn current(&self, access_token: &str) -> Result<EvidenceTrust, ApplicationError> {
        self.identity.authenticate(access_token)?;
        let individual_signing = match &self.reader {
            Some(reader) => reader.current()?,
            None => None,
        };
        Ok(EvidenceTrust {
            material: self.material.clone(),
            individual_signing,
        })
    }
}
