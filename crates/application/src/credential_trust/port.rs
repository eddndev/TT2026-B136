use super::{CredentialTrustExpectation, CredentialTrustSnapshot};
use crate::ApplicationError;
use domain::crypto::CredentialTrustInspection;

pub trait CredentialTrustPublisher: Send + Sync {
    fn publish(
        &self,
        root: &[u8],
        crl: &[u8],
        expected: CredentialTrustExpectation,
    ) -> Result<CredentialTrustSnapshot, ApplicationError>;
}
/// Publication requires administrative credentials; runtime has read access only.
pub trait CredentialTrustStore: Send + Sync {
    fn publish(
        &self,
        expected: CredentialTrustExpectation,
        inspection: CredentialTrustInspection,
    ) -> Result<CredentialTrustSnapshot, ApplicationError>;
}
