use std::io::Read;

use application::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    document_signing::{SigningAccountContext, SigningBindingReceipt},
    identity::Principal,
};
use domain::{
    clock::OffsetDateTime,
    crypto::{
        CertificateSummary, CredentialCertificate, CredentialCheck, CredentialTrustInspection,
        DocumentHasher, Sha256Digest, Signature,
    },
    document_signing::{SignerAccount, SigningBindingMaterial, SigningBindingStatement},
    identity::{Role, UserId},
    DomainError,
};
use uuid::Uuid;

pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        // This deterministic double exercises orchestration, not cryptography.
        let mut digest = [0_u8; 32];
        for (i, byte) in bytes.iter().enumerate() {
            digest[i % 32] = digest[i % 32].wrapping_add(*byte).wrapping_add(i as u8);
        }
        Sha256Digest::from_array(digest)
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("binding uses bounded public bytes")
    }
}
pub fn at(value: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(value).unwrap()
}
pub fn id() -> Uuid {
    Uuid::from_u128(44)
}
pub fn principal() -> Principal {
    Principal {
        id: UserId::from_uuid(Uuid::from_u128(1)),
        email: "litigator@example.test".into(),
        role: Role::Litigator,
    }
}
pub fn account() -> SignerAccount {
    SignerAccount::new(principal().id, Role::Litigator, true, 9, 4).unwrap()
}
pub fn certificate() -> CredentialCertificate {
    let der = b"synthetic-signing-certificate".to_vec();
    CredentialCertificate {
        fingerprint: Hasher.hash_bytes(&der),
        der,
        summary: CertificateSummary {
            subject: "Synthetic Signer".into(),
            issuer: "Synthetic Root".into(),
            serial_hex: "1000".into(),
            not_before_unix: 50,
            not_after_unix: 2500,
        },
    }
}
pub fn trust() -> CredentialTrustSnapshot {
    CredentialTrustSnapshot {
        deployment_id: Uuid::from_u128(11),
        revision: CredentialTrustRevision::new(7).unwrap(),
        inspection: CredentialTrustInspection {
            root_der: b"root".to_vec(),
            crl_der: b"crl".to_vec(),
            root_fingerprint: Hasher.hash_bytes(b"root"),
            crl_digest: Hasher.hash_bytes(b"crl"),
            crl_number: 8,
            crl_this_update: 100,
            crl_next_update: 2000,
            valid_from: 100,
            valid_until: 2000,
        },
        published_at: at(100),
        published_by: "admin".into(),
    }
}
pub fn context() -> SigningAccountContext {
    SigningAccountContext {
        principal: principal(),
        account: account(),
        trust: Some(trust()),
    }
}
pub fn statement() -> SigningBindingStatement {
    let trust = trust();
    SigningBindingStatement::new(
        account(),
        principal().id,
        SigningBindingMaterial::new(
            trust.deployment_id,
            id(),
            trust.inspection.root_fingerprint,
            certificate().fingerprint,
            trust.revision.get(),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn signature() -> Signature {
    Signature::from_bytes(vec![5; 384]).unwrap()
}
pub fn check(
    statement: &SigningBindingStatement,
    signature: &Signature,
    trust: &CredentialTrustSnapshot,
    at: i64,
) -> CredentialCheck {
    CredentialCheck {
        certificate: certificate(),
        trust: trust.inspection.clone(),
        statement_digest: Hasher.hash_bytes(&statement.canonical_bytes()),
        signature: signature.clone(),
        checked_at: at,
        valid_from: 100,
        valid_until: 2000,
    }
}
pub fn receipt() -> SigningBindingReceipt {
    SigningBindingReceipt {
        statement: statement(),
        check: check(&statement(), &signature(), &trust(), 1000),
        trust: trust(),
        registered_at: at(1000),
        withdrawn_at: None,
    }
}
