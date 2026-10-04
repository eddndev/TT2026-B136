use application::{
    credential_trust::{CredentialTrustRevision, CredentialTrustSnapshot},
    identity::{
        certificate_login::CertificateLoginContext, owner_certificates::OwnerBindingAccount,
        Principal,
    },
};
use base64::{engine::general_purpose::STANDARD, Engine};
use domain::{
    clock::OffsetDateTime,
    crypto::{
        CertificateSummary, CredentialCertificate, CredentialTrustInspection, DocumentHasher,
    },
    identity::Role,
    owner_certificate_login::{LoginAccount, LoginNonce, LoginStatement},
    owner_certificates::BindingMaterial,
};
use infrastructure::RingSha256Hasher;
use serde_json::{json, Value};

use super::support::*;

/// Synthetic public material exercises transport; no RSA or X.509 admission is asserted.
pub fn capture(issued: i64, duration: i64) -> StoredCertificateLogin {
    let context = CertificateLoginContext {
        account: OwnerBindingAccount {
            principal: Principal {
                id: UserId::new(),
                email: "owner-runtime@example.test".into(),
                role: Role::Owner,
            },
            active: true,
            revision: i64::MAX as u64,
            auth_generation: 9_007_199_254_740_993,
        },
        binding_id: Uuid::new_v4(),
        certificate: CredentialCertificate {
            der: b"synthetic-partner-der".to_vec(),
            fingerprint: RingSha256Hasher.hash_bytes(b"synthetic-partner-der"),
            summary: CertificateSummary {
                subject: "Synthetic Partner".into(),
                issuer: "Synthetic Root".into(),
                serial_hex: "1000".into(),
                not_before_unix: issued - 60,
                not_after_unix: issued + 7200,
            },
        },
        trust: CredentialTrustSnapshot {
            deployment_id: Uuid::new_v4(),
            revision: CredentialTrustRevision::new(7).unwrap(),
            inspection: CredentialTrustInspection {
                root_der: b"synthetic-root".to_vec(),
                crl_der: b"synthetic-crl".to_vec(),
                root_fingerprint: RingSha256Hasher.hash_bytes(b"synthetic-root"),
                crl_digest: RingSha256Hasher.hash_bytes(b"synthetic-crl"),
                crl_number: u64::MAX,
                crl_this_update: issued - 60,
                crl_next_update: issued + 3600,
                valid_from: issued - 60,
                valid_until: issued + 3600,
            },
            published_at: OffsetDateTime::from_unix_timestamp(issued - 60)
                .unwrap()
                .replace_nanosecond(123_456_789)
                .unwrap(),
            published_by: "synthetic-publisher".into(),
        },
    };
    let statement = statement(
        &context,
        LoginNonce::from_bytes(&[17; 32]).unwrap(),
        issued,
        duration,
    );
    StoredCertificateLogin { context, statement }
}

pub fn statement(
    context: &CertificateLoginContext,
    nonce: LoginNonce,
    issued: i64,
    duration: i64,
) -> LoginStatement {
    let account = &context.account;
    LoginStatement::new(
        LoginAccount::new(
            account.principal.id,
            account.principal.role,
            account.active,
            account.auth_generation,
        )
        .unwrap(),
        BindingMaterial::new(
            context.trust.deployment_id,
            context.binding_id,
            context.trust.inspection.root_fingerprint,
            context.certificate.fingerprint,
            context.trust.revision.get(),
        )
        .unwrap(),
        nonce,
        issued,
        issued + duration,
    )
    .unwrap()
}

pub fn wire(value: &StoredCertificateLogin) -> Value {
    let context = &value.context;
    let account = &context.account;
    let certificate = &context.certificate;
    let summary = &certificate.summary;
    let trust = &context.trust;
    let inspection = &trust.inspection;
    json!({
        "version": 1,
        "context": {
            "account": { "principal": account.principal, "active": account.active,
                "revision": account.revision, "auth_generation": account.auth_generation },
            "binding_id": context.binding_id,
            "certificate": { "der": STANDARD.encode(&certificate.der), "fingerprint": certificate.fingerprint.to_hex(),
                "summary": { "subject": summary.subject, "issuer": summary.issuer, "serial_hex": summary.serial_hex,
                    "not_before_unix": summary.not_before_unix, "not_after_unix": summary.not_after_unix } },
            "trust": {
                "deployment_id": trust.deployment_id, "revision": trust.revision.get(),
                "inspection": { "root_der": STANDARD.encode(&inspection.root_der), "crl_der": STANDARD.encode(&inspection.crl_der),
                    "root_fingerprint": inspection.root_fingerprint.to_hex(), "crl_digest": inspection.crl_digest.to_hex(),
                    "crl_number": inspection.crl_number, "crl_this_update": inspection.crl_this_update,
                    "crl_next_update": inspection.crl_next_update, "valid_from": inspection.valid_from, "valid_until": inspection.valid_until },
                "published_at": { "seconds": trust.published_at.unix_timestamp(), "nanoseconds": trust.published_at.nanosecond() },
                "published_by": trust.published_by,
            },
        },
        "statement": STANDARD.encode(value.statement.canonical_bytes()),
    })
}
