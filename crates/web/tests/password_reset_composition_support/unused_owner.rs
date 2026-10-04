use std::{io::Read, sync::Arc};

use application::{
    credential_trust::CredentialTrustSnapshot, identity::owner_certificates::*, ApplicationError,
};
use domain::{
    clock::{Clock, OffsetDateTime},
    crypto::{
        CredentialCertificate, CredentialCheck, CredentialFailure, DocumentHasher, Sha256Digest,
        Signature,
    },
    identity::UserId,
    owner_certificates::BindingStatement,
    DomainError,
};
use uuid::Uuid;

struct Unused;

pub(super) fn service() -> Arc<OwnerCertificateService> {
    let ports = Arc::new(Unused);
    Arc::new(OwnerCertificateService::new(OwnerCertificatePorts {
        identity: Arc::new(super::identity::StubIdentity),
        store: ports.clone(),
        verifier: ports.clone(),
        hasher: ports.clone(),
        clock: ports,
    }))
}

impl OwnerCertificateStore for Unused {
    fn load_registration(&self, _: UserId) -> Result<OwnerRegistrationContext, ApplicationError> {
        unreachable!("composition fixture does not prepare Owner certificates")
    }
    fn find(&self, _: UserId, _: Uuid) -> Result<Option<OwnerBindingReceipt>, ApplicationError> {
        unreachable!("composition fixture does not query Owner certificates")
    }
    fn commit_registration(
        &self,
        _: VerifiedOwnerRegistration,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        unreachable!("composition fixture does not register Owner certificates")
    }
    fn load_withdrawal(
        &self,
        _: UserId,
        _: Uuid,
    ) -> Result<OwnerWithdrawalContext, ApplicationError> {
        unreachable!("composition fixture does not prepare Owner withdrawal")
    }
    fn commit_withdrawal(
        &self,
        _: PreparedOwnerWithdrawal,
    ) -> Result<OwnerBindingCommit, ApplicationError> {
        unreachable!("composition fixture does not withdraw Owner certificates")
    }
}

impl OwnerBindingVerifier for Unused {
    fn inspect_certificate(&self, _: &[u8]) -> Result<CredentialCertificate, CredentialFailure> {
        unreachable!("composition fixture does not inspect Owner certificates")
    }
    fn verify_registration(
        &self,
        _: &BindingStatement,
        _: &[u8],
        _: &Signature,
        _: &CredentialTrustSnapshot,
        _: i64,
    ) -> Result<CredentialCheck, OwnerCertificateError> {
        unreachable!("composition fixture does not verify Owner certificates")
    }
}

impl DocumentHasher for Unused {
    fn hash_bytes(&self, _: &[u8]) -> Sha256Digest {
        unreachable!("composition fixture does not hash Owner statements")
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        unreachable!("composition fixture does not stream Owner material")
    }
}

impl Clock for Unused {
    fn now(&self) -> OffsetDateTime {
        unreachable!("composition fixture does not time Owner operations")
    }
}
