use std::io::Read;

use application::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    identity::{owner_certificates::*, Principal},
};
use domain::{
    clock::OffsetDateTime,
    crypto::{
        CertificateSummary, CredentialCertificate, CredentialCheck, CredentialTrustInspection,
        DocumentHasher, Sha256Digest, Signature,
    },
    identity::{Role, UserId},
    owner_certificates::{BindingMaterial, BindingRecord, BindingStatement, OwnerAccount},
    DomainError,
};
use uuid::Uuid;

pub const RAW_CERTIFICATE: &[u8] = b"synthetic-public-certificate-input";

pub struct Hasher;
impl DocumentHasher for Hasher {
    fn hash_bytes(&self, data: &[u8]) -> Sha256Digest {
        // Deterministic port double; this test does not exercise cryptography.
        let mut value = [0_u8; 32];
        for (index, byte) in data.iter().enumerate() {
            value[index % 32] = value[index % 32]
                .wrapping_add(*byte)
                .wrapping_add(index as u8);
        }
        Sha256Digest::from_array(value)
    }
    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("owner binding workflow must hash bounded canonical bytes")
    }
}

pub fn at(value: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(value).unwrap()
}

pub fn binding() -> Uuid {
    Uuid::from_u128(44)
}

pub fn principal() -> Principal {
    Principal {
        id: UserId::from_uuid(Uuid::from_u128(1)),
        email: "owner@example.test".into(),
        role: Role::Owner,
    }
}

pub fn account() -> OwnerBindingAccount {
    OwnerBindingAccount {
        principal: principal(),
        active: true,
        revision: 9,
        auth_generation: 4,
    }
}

pub fn trust() -> CredentialTrustSnapshot {
    CredentialTrustSnapshot {
        deployment_id: Uuid::from_u128(11),
        revision: CredentialTrustRevision::new(7).unwrap(),
        inspection: CredentialTrustInspection {
            root_der: b"synthetic-root".to_vec(),
            crl_der: b"synthetic-crl".to_vec(),
            root_fingerprint: Hasher.hash_bytes(b"synthetic-root"),
            crl_digest: Hasher.hash_bytes(b"synthetic-crl"),
            crl_number: 8,
            crl_this_update: 100,
            crl_next_update: 2000,
            valid_from: 100,
            valid_until: 2000,
        },
        published_at: at(100),
        published_by: "synthetic-admin".into(),
    }
}

pub fn context() -> OwnerRegistrationContext {
    OwnerRegistrationContext {
        account: account(),
        trust: Some(trust()),
    }
}

pub fn certificate() -> CredentialCertificate {
    CredentialCertificate {
        der: b"synthetic-partner-der".to_vec(),
        fingerprint: Hasher.hash_bytes(b"synthetic-partner-der"),
        summary: CertificateSummary {
            subject: "Synthetic Partner".into(),
            issuer: "Synthetic Root".into(),
            serial_hex: "1000".into(),
            not_before_unix: 50,
            not_after_unix: 2500,
        },
    }
}

pub fn statement() -> BindingStatement {
    let account = account();
    let trust = trust();
    BindingStatement::new(
        OwnerAccount::new(account.principal.id, Role::Owner, true, 9, 4).unwrap(),
        account.principal.id,
        BindingMaterial::new(
            trust.deployment_id,
            binding(),
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
    statement: &BindingStatement,
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

pub fn receipt() -> OwnerBindingReceipt {
    OwnerBindingReceipt {
        owner: principal().id,
        record: BindingRecord::registered(statement()),
        check: check(&statement(), &signature(), &trust(), 1000),
        trust: trust(),
        registered_at: at(1000),
        withdrawn_at: None,
    }
}

pub fn retired() -> OwnerBindingReceipt {
    let mut receipt = receipt();
    receipt.record = receipt
        .record
        .withdraw(
            OwnerAccount::new(principal().id, Role::Owner, true, 10, 4).unwrap(),
            1,
        )
        .unwrap();
    receipt.withdrawn_at = Some(at(3000));
    receipt
}
