use std::io::Read;

use application::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    identity::{
        certificate_login::CertificateLoginContext, owner_certificates::OwnerBindingAccount,
        Principal, UserRecord,
    },
};
use domain::{
    clock::OffsetDateTime,
    crypto::{
        CertificateSummary, CredentialCertificate, CredentialTrustInspection, DocumentHasher,
        RecoveryCodeSet, Sha256Digest, RECOVERY_CODE_COUNT,
    },
    identity::{Role, UserId},
    DomainError,
};
use uuid::Uuid;

pub struct Hasher;

impl DocumentHasher for Hasher {
    fn hash_bytes(&self, bytes: &[u8]) -> Sha256Digest {
        // This deterministic double checks port contracts, not cryptography.
        let mut digest = [0_u8; 32];
        for (index, byte) in bytes.iter().enumerate() {
            digest[index % 32] = digest[index % 32]
                .wrapping_add(*byte)
                .wrapping_add(index as u8);
        }
        Sha256Digest::from_array(digest)
    }

    fn hash_stream(&self, _: &mut dyn Read) -> Result<Sha256Digest, DomainError> {
        panic!("login admission must hash bounded material")
    }
}

pub fn at(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}

pub fn binding() -> Uuid {
    Uuid::from_u128(44)
}

pub fn user() -> UserRecord {
    UserRecord {
        id: UserId::from_uuid(Uuid::from_u128(1)),
        email: "owner@example.test".into(),
        role: Role::Owner,
        active: true,
        revision: 9,
        auth_generation: 4,
        password_hash: "fake:password".into(),
        protected_totp_secret: vec![7; 20],
        recovery_codes: RecoveryCodeSet::from_hashes(
            (0..RECOVERY_CODE_COUNT)
                .map(|index| format!("fake:RECOVERY-{index}"))
                .collect(),
        )
        .unwrap(),
    }
}

pub fn context() -> CertificateLoginContext {
    let user = user();
    CertificateLoginContext {
        account: OwnerBindingAccount {
            principal: Principal::from(&user),
            active: true,
            revision: user.revision,
            auth_generation: user.auth_generation,
        },
        binding_id: binding(),
        certificate: CredentialCertificate {
            der: b"synthetic-partner-der".to_vec(),
            fingerprint: Hasher.hash_bytes(b"synthetic-partner-der"),
            summary: CertificateSummary {
                subject: "Synthetic Partner".into(),
                issuer: "Synthetic Root".into(),
                serial_hex: "1000".into(),
                not_before_unix: 50,
                not_after_unix: 2500,
            },
        },
        trust: CredentialTrustSnapshot {
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
        },
    }
}
